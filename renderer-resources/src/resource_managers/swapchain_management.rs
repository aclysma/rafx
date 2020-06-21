use fnv::FnvHashMap;
use renderer_assets::vk_description as dsc;
use crate::resource_managers::asset_lookup::{LoadedAssetLookupSet, LoadedMaterial};
use crate::resource_managers::resource_lookup::ResourceLookupSet;
use renderer_assets::vk_description::SwapchainSurfaceInfo;
use ash::prelude::*;

pub struct ActiveSwapchainSurfaceInfoState {
    pub ref_count: u32,
    pub index: usize,
}

#[derive(Default)]
pub struct ActiveSwapchainSurfaceInfoSet {
    pub ref_counts: FnvHashMap<dsc::SwapchainSurfaceInfo, ActiveSwapchainSurfaceInfoState>,

    //TODO: Could make this a slab which would persist indexes across frames
    pub unique_swapchain_infos: Vec<dsc::SwapchainSurfaceInfo>,
}

impl ActiveSwapchainSurfaceInfoSet {
    fn add_material_for_swapchain(
        //&mut self,
        resources: &mut ResourceLookupSet,
        swapchain_surface_info: &SwapchainSurfaceInfo,
        loaded_material: &mut LoadedMaterial,
    ) -> VkResult<()> {
        for pass in &mut loaded_material.passes {
            let pipeline = resources.get_or_create_graphics_pipeline(
                &pass.pipeline_create_data,
                swapchain_surface_info,
            )?;

            pass.render_passes.push(pipeline.get_raw().renderpass);
            pass.pipelines.push(pipeline);
        }

        Ok(())
    }

    pub fn add(
        &mut self,
        swapchain_surface_info: &dsc::SwapchainSurfaceInfo,
        loaded_assets: &mut LoadedAssetLookupSet,
        resources: &mut ResourceLookupSet,
    ) -> VkResult<()> {
        let added_swapchain = match self.ref_counts.get_mut(swapchain_surface_info) {
            Some(state) => {
                state.ref_count += 1;
                false
            }
            None => {
                &self.ref_counts.insert(
                    swapchain_surface_info.clone(),
                    ActiveSwapchainSurfaceInfoState {
                        ref_count: 1,
                        index: self.unique_swapchain_infos.len(),
                    },
                );

                self.unique_swapchain_infos
                    .push(swapchain_surface_info.clone());
                true
            }
        };

        if added_swapchain {
            for (load_handle, loaded_asset) in &mut loaded_assets.materials.loaded_assets {
                if let Some(committed) = &mut loaded_asset.committed {
                    Self::add_material_for_swapchain(resources, swapchain_surface_info, committed)?;
                }

                if let Some(uncommitted) = &mut loaded_asset.uncommitted {
                    Self::add_material_for_swapchain(
                        resources,
                        swapchain_surface_info,
                        uncommitted,
                    )?;
                }
            }
        }

        Ok(())
    }

    pub fn remove(
        &mut self,
        swapchain_surface_info: &dsc::SwapchainSurfaceInfo,
        loaded_assets: &mut LoadedAssetLookupSet,
    ) {
        let remove_index = match self.ref_counts.get_mut(swapchain_surface_info) {
            Some(state) => {
                if state.ref_count == 1 {
                    let removed_index = state.index;
                    self.ref_counts.remove(swapchain_surface_info);

                    for (x, mut y) in &mut self.ref_counts {
                        if y.index > removed_index {
                            y.index -= 1;
                        }
                    }
                    self.unique_swapchain_infos.swap_remove(removed_index);
                    Some(removed_index)
                } else {
                    None
                }
            }
            // If it doesn't exist, then a remove call was made before a matching add call
            None => unreachable!(),
        };

        //TODO: Common case is to destroy and re-create the same swapchain surface info, so we can
        // delay destroying until we also get an additional add/remove. If the next add call is
        // the same, we can avoid the remove entirely
        if let Some(remove_index) = remove_index {
            for (_, loaded_asset) in &mut loaded_assets.materials.loaded_assets {
                if let Some(committed) = &mut loaded_asset.committed {
                    for pass in &mut committed.passes {
                        pass.render_passes.swap_remove(remove_index);
                        pass.pipelines.swap_remove(remove_index);
                    }
                }

                if let Some(uncommitted) = &mut loaded_asset.uncommitted {
                    for pass in &mut uncommitted.passes {
                        pass.render_passes.swap_remove(remove_index);
                        pass.pipelines.swap_remove(remove_index);
                    }
                }
            }
        } else {
            log::error!(
                "Received a remove swapchain without a matching add\n{:#?}",
                swapchain_surface_info
            );
        }
    }

    pub fn unique_swapchain_infos(&self) -> &Vec<dsc::SwapchainSurfaceInfo> {
        &self.unique_swapchain_infos
    }
}
