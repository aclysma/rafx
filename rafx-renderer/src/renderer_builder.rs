use super::daemon::AssetDaemonOpt;
use super::{daemon, Renderer};
use super::{RenderGraphGenerator, RendererPlugin};
use rafx_api::{RafxApi, RafxQueueType, RafxResult};
use rafx_assets::distill_impl::AssetResource;
use rafx_assets::{AssetManager, UploadQueueConfig};
use rafx_framework::nodes::{ExtractResources, RenderRegistryBuilder};

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

#[derive(Default)]
pub struct RendererBuilder {
    plugins: Vec<Box<dyn RendererPlugin>>,
}

impl RendererBuilder {
    pub fn add_plugin(
        mut self,
        plugin: Box<dyn RendererPlugin>,
    ) -> Self {
        self.plugins.push(plugin);
        self
    }

    pub fn build(
        self,
        extract_resources: ExtractResources,
        rafx_api: &RafxApi,
        asset_source: AssetSource,
        render_graph_generator: Box<dyn RenderGraphGenerator>,
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

                    for plugin in &self.plugins {
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
        for plugin in &self.plugins {
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

        for plugin in &self.plugins {
            plugin.register_asset_types(&mut asset_manager, &mut asset_resource);
        }

        let renderer = Renderer::new(
            extract_resources,
            &mut asset_resource,
            &mut asset_manager,
            &graphics_queue,
            &transfer_queue,
            self.plugins,
            render_graph_generator,
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
