use super::Renderer;
use super::{RenderFeaturePlugin, RendererPipelinePlugin};
use crate::renderer_thread_pool_none::RendererThreadPoolNone;
use crate::{RendererAssetPlugin, RendererThreadPool};
use fnv::FnvHashSet;
use rafx_api::{RafxApi, RafxQueueType, RafxResult};
use rafx_assets::AssetManager;
use rafx_assets::AssetResource;
use rafx_framework::render_features::{ExtractResources, RenderRegistryBuilder};
use rafx_framework::upload::UploadQueueConfig;
use rafx_framework::RenderResources;
use std::sync::Arc;

pub enum AssetSource {
    BuildDir(std::path::PathBuf),
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
    pub fn add_render_feature_plugin(
        mut self,
        plugin: Arc<dyn RenderFeaturePlugin>,
    ) -> Self {
        self.feature_plugins.push(plugin);
        self
    }

    pub fn add_asset_plugin(
        mut self,
        plugin: Arc<dyn RendererAssetPlugin>,
    ) -> Self {
        self.asset_plugins.push(plugin);
        self
    }

    pub fn allow_use_render_thread(
        mut self,
        allow_use_render_thread: bool,
    ) -> Self {
        self.allow_use_render_thread = allow_use_render_thread;
        self
    }

    pub fn build(
        self,
        extract_resources: ExtractResources,
        rafx_api: &RafxApi,
        asset_source: AssetSource,
        pipeline_plugin: Arc<dyn RendererPipelinePlugin>,
        renderer_thread_pool: fn() -> Option<Box<dyn RendererThreadPool>>, // TODO(dvd): Change to threading type enum with options None, RenderThread, or ThreadPool.
    ) -> RafxResult<RendererBuilderResult> {
        let mut asset_resource = match asset_source {
            AssetSource::BuildDir(build_dir) => {
                log::info!("Renderer build dir: {:?}", build_dir);
                AssetResource::new(build_dir).unwrap()
            }
        };
        // let mut asset_resource = match asset_source {
        //     AssetSource::Packfile(packfile) => {
        //         log::info!("Reading from packfile {:?}", packfile);
        //
        //         // Initialize the packfile loader with the packfile path
        //         daemon::init_distill_packfile(&packfile)
        //     }
        //     AssetSource::Daemon {
        //         external_daemon,
        //         daemon_args,
        //     } => {
        //         if !external_daemon {
        //             log::info!("Hosting local daemon at {:?}", daemon_args.address);
        //
        //             let mut asset_dirs = FnvHashSet::default();
        //             for path in daemon_args.asset_dirs {
        //                 log::info!("Added asset path {:?}", path);
        //                 asset_dirs.insert(path);
        //             }
        //
        //             for plugin in &self.asset_plugins {
        //                 let mut paths = Default::default();
        //                 plugin.add_asset_paths(&mut paths);
        //                 for path in paths {
        //                     log::info!(
        //                         "Added asset path {:?} from asset plugin {}",
        //                         path,
        //                         plugin.plugin_name()
        //                     );
        //                     asset_dirs.insert(path);
        //                 }
        //             }
        //
        //             for plugin in &self.feature_plugins {
        //                 let mut paths = Default::default();
        //                 plugin.add_asset_paths(&mut paths);
        //                 for path in paths {
        //                     log::info!(
        //                         "Added asset path {:?} from feature plugin {:?}",
        //                         path,
        //                         plugin.feature_debug_constants().feature_name
        //                     );
        //                     asset_dirs.insert(path);
        //                 }
        //             }
        //
        //             {
        //                 let mut paths = Default::default();
        //                 pipeline_plugin.add_asset_paths(&mut paths);
        //                 for path in paths {
        //                     log::info!(
        //                         "Added asset path {:?} from pipeline plugin {:?}",
        //                         path,
        //                         pipeline_plugin.plugin_name()
        //                     );
        //                     asset_dirs.insert(path);
        //                 }
        //             }
        //
        //             let mut asset_daemon = rafx_assets::hydrate_impl::default_daemon()
        //                 .with_db_path(daemon_args.db_dir)
        //                 .with_address(daemon_args.address)
        //                 .with_asset_dirs(asset_dirs.into_iter().collect());
        //
        //             for plugin in &self.asset_plugins {
        //                 asset_daemon = plugin.configure_asset_daemon(asset_daemon);
        //             }
        //
        //             // Spawn the daemon in a background thread.
        //             std::thread::spawn(move || {
        //                 asset_daemon.run();
        //             });
        //         } else {
        //             log::info!("Connecting to daemon at {:?}", daemon_args.address);
        //         }
        //
        //         // Connect to the daemon we just launched
        //         daemon::init_distill_daemon(daemon_args.address.to_string())
        //     }
        // };

        let mut render_registry_builder = RenderRegistryBuilder::default();
        for plugin in &self.feature_plugins {
            render_registry_builder = plugin.configure_render_registry(render_registry_builder);
        }
        for plugin in &self.asset_plugins {
            render_registry_builder = plugin.configure_render_registry(render_registry_builder);
        }
        render_registry_builder =
            pipeline_plugin.configure_render_registry(render_registry_builder);

        let render_registry = render_registry_builder.build();

        let device_context = rafx_api.device_context();

        let graphics_queue = device_context.create_queue(RafxQueueType::Graphics)?;
        let transfer_queue = device_context.create_queue(RafxQueueType::Transfer)?;

        let mut asset_manager = AssetManager::new(
            &device_context,
            &render_registry,
            UploadQueueConfig {
                max_concurrent_uploads: 2,
                max_new_uploads_in_single_frame: 1,
                max_bytes_per_upload: 64 * 1024 * 1024,
            },
            &graphics_queue,
            &transfer_queue,
        )?;

        let mut render_resources = RenderResources::default();

        asset_manager.register_default_asset_types(&mut asset_resource, &mut render_resources)?;

        for plugin in &self.asset_plugins {
            plugin.register_asset_types(
                &mut asset_manager,
                &mut asset_resource,
                &mut render_resources,
            )?;
        }

        let renderer = Renderer::new(
            extract_resources,
            render_resources,
            &mut asset_resource,
            &mut asset_manager,
            &graphics_queue,
            &transfer_queue,
            self.feature_plugins,
            self.asset_plugins,
            pipeline_plugin,
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
