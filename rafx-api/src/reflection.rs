use crate::types::{RafxResourceType, RafxShaderStageFlags};
use crate::{RafxResult, RafxShaderStageDef};
use fnv::FnvHashMap;
#[cfg(feature = "serde-support")]
use serde::{Deserialize, Serialize};

// #[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
// #[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
// pub enum RafxShaderResourceTextureDimension {
//     Dim1D,
//     Dim2D,
//     Dim2DMultiSample,
//     Dim3D,
//     DimCube,
//     Dim1DArray,
//     Dim2DArray,
//     Dim2DMultiSampleArray,
//     DimCubeArray,
// }
//
// impl Default for RafxShaderResourceTextureDimension {
//     fn default() -> Self {
//         RafxShaderResourceTextureDimension::Dim2D
//     }
// }

// Doesn't do anything, so commented out
// #[derive(Debug, Clone, PartialEq)]
// #[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
// pub struct RafxVertexInput {
//     pub semantic: String,
//     pub location: u32,
//     //pub location
//     //pub location:
//     //pub size: u32,
// }

#[derive(PartialEq, Eq, Hash)]
struct RafxShaderResourceBindingKey {
    set: u32,
    binding: u32,
}

//TODO: Consider separate type for bindings vs. push constants
#[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct RafxShaderResource {
    pub resource_type: RafxResourceType,
    pub set_index: u32,
    pub binding: u32,
    // Valid only for descriptors (resource_type != ROOT_CONSTANT)
    pub element_count: u32,
    // Valid only for push constants (resource_type != ROOT_CONSTANT)
    pub size_in_bytes: u32,
    pub used_in_shader_stages: RafxShaderStageFlags,
    // Name is optional
    //TODO: Add some sort of hashing-friendly option
    pub name: Option<String>,
    //pub texture_dimensions: Option<RafxTextureDimension>,
    // metal stuff?
}

impl RafxShaderResource {
    pub fn element_count_normalized(&self) -> u32 {
        // Assume 0 = default of 1
        self.element_count.max(1)
    }

    pub fn validate(&self) -> RafxResult<()> {
        if self.resource_type == RafxResourceType::ROOT_CONSTANT {
            if self.element_count != 0 {
                Err(
                    format!(
                        "binding (set={:?} binding={:?} name={:?} type={:?}) has non-zero element_count",
                        self.set_index,
                        self.binding,
                        self.name,
                        self.resource_type
                    )
                )?;
            }
            if self.size_in_bytes == 0 {
                Err(format!(
                    "binding (set={:?} binding={:?} name={:?} type={:?}) has zero size_in_bytes",
                    self.set_index, self.binding, self.name, self.resource_type
                ))?;
            }
            if self.set_index != 0 {
                Err(format!(
                    "binding (set={:?} binding={:?} name={:?} type={:?}) has non-zero set_index",
                    self.set_index, self.binding, self.name, self.resource_type
                ))?;
            }
            if self.binding != 0 {
                Err(format!(
                    "binding (set={:?} binding={:?} name={:?} type={:?}) has non-zero binding",
                    self.set_index, self.binding, self.name, self.resource_type
                ))?;
            }
        } else {
            if self.size_in_bytes != 0 {
                Err(
                    format!(
                        "binding (set={:?} binding={:?} name={:?} type={:?}) has non-zero size_in_bytes",
                        self.set_index,
                        self.binding,
                        self.name,
                        self.resource_type
                    )
                )?;
            }
        }

        Ok(())
    }

    fn binding_key(&self) -> RafxShaderResourceBindingKey {
        RafxShaderResourceBindingKey {
            set: self.set_index,
            binding: self.binding,
        }
    }

    fn verify_compatible_across_stages(
        &self,
        other: &Self,
    ) -> RafxResult<()> {
        if self.resource_type != other.resource_type {
            Err(format!(
                "Pass is using shaders in different stages with different resource_type {:?} and {:?} (set={} binding={})",
                self.resource_type, other.resource_type,
                self.set_index,
                self.binding
            ))?;
        }

        if self.element_count_normalized() != other.element_count_normalized() {
            Err(format!(
                "Pass is using shaders in different stages with different element_count {} and {} (set={} binding={})", self.element_count_normalized(), other.element_count_normalized(),
                self.set_index, self.binding
            ))?;
        }

        if self.size_in_bytes != other.size_in_bytes {
            Err(format!(
                "Pass is using shaders in different stages with different size_in_bytes {} and {} (set={} binding={})",
                self.size_in_bytes, other.size_in_bytes,
                self.set_index, self.binding
            ))?;
        }

        // if self.texture_dimensions != other.texture_dimensions {
        //     Err(format!(
        //         "Pass is using shaders in different stages with different texture_dimensions {:?} and {:?} (set={} binding={})",
        //         self.texture_dimensions, other.texture_dimensions,
        //         self.set_index,
        //         self.binding
        //     ))?;
        // }

        Ok(())
    }
}

// pub struct RafxShaderVariable {
//     parent_index: u32,
//     offset: u32,
//     size: u32,
//     name: String,
// }
//
// impl RafxShaderVariable {
//     fn binding_key(&self) -> BindingKey {
//         BindingKey {
//             set: self.set,
//             binding: self.binding,
//         }
//     }
//
//     fn verify_compatible(&self, other: &Self) -> RafxResult<()> {
//         if self.parent_index != other.parent_index {
//             return Err("Shader resource offset does not match").into();
//         }
//
//         if self.offset != other.offset {
//             return Err("Shader resource offset does not match").into();
//         }
//
//         if self.size != other.size {
//             return Err("Shader resource size does not match").into();
//         }
//
//         if self.name != other.name {
//             return Err("Shader resource name does not match").into();
//         }
//
//         Ok(())
//     }
// }

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct RafxShaderStageReflection {
    // For now, this doesn't do anything, so commented out
    //pub vertex_inputs: Vec<RafxVertexInput>,
    pub shader_stage: RafxShaderStageFlags,
    pub resources: Vec<RafxShaderResource>,
    pub thread_count: [u32; 3],
    pub entry_point_name: String,
}

#[derive(Debug)]
pub struct RafxPipelineReflection {
    pub shader_stages: RafxShaderStageFlags,
    pub resources: Vec<RafxShaderResource>,
}

impl RafxPipelineReflection {
    pub fn from_stages(stages: &[RafxShaderStageDef]) -> RafxResult<RafxPipelineReflection> {
        let mut unmerged_resources = Vec::default();
        for stage in stages {
            assert!(!stage.shader_stage.is_empty());
            for resource in &stage.resources {
                // The provided resource MAY (but does not need to) have the shader stage flag set.
                // (Leaving it default empty is fine). It will automatically be set here.
                if !(resource.used_in_shader_stages - stage.shader_stage).is_empty() {
                    let message = format!(
                        "A resource in shader stage {:?} has other stages {:?} set",
                        stage.shader_stage,
                        resource.used_in_shader_stages - stage.shader_stage
                    );
                    log::error!("{}", message);
                    Err(message)?;
                }

                let mut resource = resource.clone();
                resource.used_in_shader_stages |= stage.shader_stage;
                unmerged_resources.push(resource);
            }
        }

        log::trace!("Create RafxPipelineReflection from stages");
        let mut all_shader_stages = RafxShaderStageFlags::empty();
        for stage in stages {
            if all_shader_stages.intersects(stage.shader_stage) {
                Err(format!(
                    "Duplicate shader stage ({}) found when creating RafxPipelineReflection",
                    (all_shader_stages & stage.shader_stage).bits()
                ))?;
            }

            all_shader_stages |= stage.shader_stage;
        }

        let mut merged_resources =
            FnvHashMap::<RafxShaderResourceBindingKey, RafxShaderResource>::default();

        //TODO: Merge push constants

        //
        // Merge the resources
        //
        for resource in &unmerged_resources {
            log::trace!(
                "    Resource {:?} from stage {:?}",
                resource.name,
                resource.used_in_shader_stages
            );
            let key = resource.binding_key();
            if let Some(existing_resource) = merged_resources.get_mut(&key) {
                // verify compatible
                existing_resource.verify_compatible_across_stages(resource)?;

                log::trace!(
                    "      Already used in stages {:?} and is compatible, adding stage {:?}",
                    existing_resource.used_in_shader_stages,
                    resource.used_in_shader_stages,
                );
                existing_resource.used_in_shader_stages |= resource.used_in_shader_stages;
            } else {
                // insert it
                log::trace!(
                    "      Resource not yet used, adding it for stage {:?}",
                    resource.used_in_shader_stages
                );
                assert!(!resource.used_in_shader_stages.is_empty());
                let old = merged_resources.insert(key, resource.clone());
                assert!(old.is_none());
            }
        }

        let resources = merged_resources.into_iter().map(|(_, v)| v).collect();

        Ok(RafxPipelineReflection {
            shader_stages: all_shader_stages,
            resources,
        })
    }
}
