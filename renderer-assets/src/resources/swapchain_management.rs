use fnv::FnvHashMap;
use crate::vk_description as dsc;
use crate::resources::asset_lookup::{AssetLookupSet};
use crate::assets::{MaterialAsset, MaterialPassSwapchainResources};
use crate::resources::resource_lookup::ResourceLookupSet;
use crate::vk_description::SwapchainSurfaceInfo;
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
        loaded_material: &mut MaterialAsset,
    ) -> VkResult<()> {
        for pass in &*loaded_material.passes {
            unimplemented!();
            // let renderpass = resources.get_or_create_renderpass(pass.material_pass)
            //
            //
            // let pipeline = resources.get_or_create_graphics_pipeline(
            //     &pass.material_pass,
            //     swapchain_surface_info,
            // )?;
            //
            // let mut per_swapchain_data = pass.per_swapchain_data.lock().unwrap();
            // per_swapchain_data.push(MaterialPassSwapchainResources { pipeline });
        }

        Ok(())
    }

    pub fn add(
        &mut self,
        swapchain_surface_info: &dsc::SwapchainSurfaceInfo,
        loaded_assets: &mut AssetLookupSet,
        resources: &mut ResourceLookupSet,
    ) -> VkResult<()> {
        let added_swapchain = match self.ref_counts.get_mut(swapchain_surface_info) {
            Some(state) => {
                state.ref_count += 1;
                false
            }
            None => {
                self.ref_counts.insert(
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
            for loaded_asset in &mut loaded_assets.materials.loaded_assets.values_mut() {
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
        loaded_assets: &mut AssetLookupSet,
    ) {
        let remove_index = match self.ref_counts.get_mut(swapchain_surface_info) {
            Some(state) => {
                if state.ref_count == 1 {
                    let removed_index = state.index;
                    self.ref_counts.remove(swapchain_surface_info);

                    for y in self.ref_counts.values_mut() {
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
            for loaded_asset in &mut loaded_assets.materials.loaded_assets.values_mut() {
                if let Some(committed) = &mut loaded_asset.committed {
                    for pass in &*committed.passes {
                        let mut per_swapchain_data = pass.per_swapchain_data.lock().unwrap();
                        per_swapchain_data.swap_remove(remove_index);
                    }
                }

                if let Some(uncommitted) = &mut loaded_asset.uncommitted {
                    for pass in &*uncommitted.passes {
                        let mut per_swapchain_data = pass.per_swapchain_data.lock().unwrap();
                        per_swapchain_data.swap_remove(remove_index);
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
