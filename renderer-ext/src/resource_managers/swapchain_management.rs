use fnv::FnvHashMap;
use crate::pipeline_description as dsc;

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
    pub fn add(
        &mut self,
        swapchain_surface_info: &dsc::SwapchainSurfaceInfo,
    ) -> bool {
        match self.ref_counts.get_mut(swapchain_surface_info) {
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
        }
    }

    pub fn remove(
        &mut self,
        swapchain_surface_info: &dsc::SwapchainSurfaceInfo,
    ) -> Option<usize> {
        match self.ref_counts.get_mut(swapchain_surface_info) {
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
        }
    }

    pub fn unique_swapchain_infos(&self) -> &Vec<dsc::SwapchainSurfaceInfo> {
        &self.unique_swapchain_infos
    }
}
