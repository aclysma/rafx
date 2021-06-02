use super::daemon::AssetDaemonOpt;
use super::{daemon, Renderer};
use super::{RenderFeaturePlugin, RenderGraphGenerator};
use crate::renderer_thread_pool_none::RendererThreadPoolNone;
use crate::{RendererAssetPlugin, RendererThreadPool};
use rafx_api::{RafxApi, RafxQueueType, RafxResult};
use rafx_assets::distill_impl::AssetResource;
use rafx_assets::{AssetManager, UploadQueueConfig};
use rafx_framework::render_features::{ExtractResources, RenderRegistryBuilder};
use std::sync::Arc;

pub enum AssetSource {
    Packfile(std::path::PathBuf),
    Daemon {
        external_daemon: bool,
        daemon_args: AssetDaemonOpt,
    },
}

pub struct RendererBuilderResult {
    pub asset_resource: AssetResource,
    pub asset_manager: AssetManager,
    pub renderer: Renderer,
}

pub struct RendererBuilder {
    feature_plugins: Vec<Arc<dyn RenderFeaturePlugin>>,
    asset_plugins: Vec<Arc<dyn RendererAssetPlugin>>,
    allow_use_render_thread: bool,
}

impl Default for RendererBuilder {
    fn default() -> Self {
        RendererBuilder {
            feature_plugins: Default::default(),
            asset_plugins: Default::default(),
            allow_use_render_thread: true,
        }
    }
}

impl RendererBuilder {
    pub fn add_render_feature(
        mut self,
        plugin: Arc<dyn RenderFeaturePlugin>,
    ) -> Self {
        self.feature_plugins.push(plugin);
        self
    }

    pub fn add_asset(
        mut self,
        plugin: Arc<dyn RendererAssetPlugin>,
    ) -> Self {
        self.asset_plugins.push(plugin);
        self
    }

    pub fn allow_use_render_thread(mut self, allow_use_render_thread: bool) -> Self {
        self.allow_use_render_thread = allow_use_render_thread;
        self
    }

    pub fn build(
        self,
        extract_resources: ExtractResources,
        rafx_api: &RafxApi,
        asset_source: AssetSource,
        render_graph_generator: Box<dyn RenderGraphGenerator>,
        renderer_thread_pool: fn() -> Option<Box<dyn RendererThreadPool>>, // TODO(dvd): Change to threading type enum with options None, RenderThread, or ThreadPool.
    ) -> RafxResult<RendererBuilderResult> {
        let mut asset_resource = match asset_source {
            AssetSource::Packfile(packfile) => {
                log::info!("Reading from packfile {:?}", packfile);

                // Initialize the packfile loader with the packfile path
                daemon::init_distill_packfile(&packfile)
            }
            AssetSource::Daemon {
                external_daemon,
                daemon_args,
            } => {
                if !external_daemon {
                    log::info!("Hosting local daemon at {:?}", daemon_args.address);

                    let mut asset_daemon = rafx_assets::distill_impl::default_daemon()
                        .with_db_path(daemon_args.db_dir)
                        .with_address(daemon_args.address)
                        .with_asset_dirs(daemon_args.asset_dirs);

                    for plugin in &self.asset_plugins {
                        asset_daemon = plugin.configure_asset_daemon(asset_daemon);
                    }

                    // Spawn the daemon in a background thread.
                    std::thread::spawn(move || {
                        asset_daemon.run();
                    });
                } else {
                    log::info!("Connecting to daemon at {:?}", daemon_args.address);
                }

                // Connect to the daemon we just launched
                daemon::init_distill_daemon(daemon_args.address.to_string())
            }
        };

        let mut render_registry_builder = RenderRegistryBuilder::default();
        for plugin in &self.feature_plugins {
            render_registry_builder = plugin.configure_render_registry(render_registry_builder);
        }
        for plugin in &self.asset_plugins {
            render_registry_builder = plugin.configure_render_registry(render_registry_builder);
        }

        let render_registry = render_registry_builder.build();

        let device_context = rafx_api.device_context();

        let graphics_queue = device_context.create_queue(RafxQueueType::Graphics)?;
        let transfer_queue = device_context.create_queue(RafxQueueType::Transfer)?;

        let mut asset_manager = AssetManager::new(
            &device_context,
            &render_registry,
            UploadQueueConfig {
                max_concurrent_uploads: 4,
                max_new_uploads_in_single_frame: 4,
                max_bytes_per_upload: 64 * 1024 * 1024,
            },
            &graphics_queue,
            &transfer_queue,
        );

        asset_manager.register_default_asset_types(&mut asset_resource);

        for plugin in &self.asset_plugins {
            plugin.register_asset_types(&mut asset_manager, &mut asset_resource);
        }

        let renderer = Renderer::new(
            extract_resources,
            &mut asset_resource,
            &mut asset_manager,
            &graphics_queue,
            &transfer_queue,
            self.feature_plugins,
            self.asset_plugins,
            render_graph_generator,
            renderer_thread_pool()
                .or_else(|| Some(Box::new(RendererThreadPoolNone::new())))
                .unwrap(),
            self.allow_use_render_thread,
        );

        match renderer {
            Ok(renderer) => Ok(RendererBuilderResult {
                asset_resource,
                asset_manager,
                renderer,
            }),
            Err(e) => {
                std::mem::drop(asset_resource);
                std::mem::drop(asset_manager);
                Err(e)
            }
        }
    }
}
