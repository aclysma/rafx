use crate::types::{RafxResourceType, RafxShaderStageFlags};
use crate::{RafxResult, RafxShaderStageDef, MAX_DESCRIPTOR_SET_LAYOUTS};
use fnv::FnvHashMap;
#[cfg(feature = "serde-support")]
use serde::{Deserialize, Serialize};

/// Indicates where a resource is bound
#[derive(PartialEq, Eq, Hash, Default)]
pub struct RafxShaderResourceBindingKey {
    pub set: u32,
    pub binding: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct RafxGlUniformMember {
    pub name: String,
    pub offset: u32,
}

impl RafxGlUniformMember {
    pub fn new<T: Into<String>>(
        name: T,
        offset: u32,
    ) -> Self {
        RafxGlUniformMember {
            name: name.into(),
            offset,
        }
    }
}

/// A data source within a shader. Often a descriptor or push constant.
///
/// A RafxShaderResource may be specified by hand or generated using rafx-shader-processor
//TODO: Consider separate type for bindings vs. push constants
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct RafxShaderResource {
    pub resource_type: RafxResourceType,
    pub set_index: u32,
    pub binding: u32,
    // Valid only for descriptors (resource_type != ROOT_CONSTANT)
    // This must remain pub to init the struct as "normal" but in general,
    // access it via element_count_normalized(). This ensures that if it
    // is default-initialized to 0, it is treated as 1
    pub element_count: u32,
    // Valid only for push constants (resource_type == ROOT_CONSTANT)
    pub size_in_bytes: u32,
    pub used_in_shader_stages: RafxShaderStageFlags,
    // Name is optional
    //TODO: Add some sort of hashing-friendly option
    pub name: Option<String>,
    //pub texture_dimensions: Option<RafxTextureDimension>,
    // metal stuff?

    //TODO: Generate MSL buffer IDs offline rather than when creating root signature?
    // What we do now works but requires shader's argument buffer assignments to be assigned in a
    // very specific way. Would be better if user could provide the argument buffer ID

    // HLSL-specific binding info
    pub dx12_reg: Option<u32>,
    pub dx12_space: Option<u32>,

    // Required for GL ES (2.0/3.0) only. Other APIs use set_index and binding. (Rafx shader processor
    // can produce this metadata automatically)
    pub gles_name: Option<String>,

    // Required for GL ES (2.0/3.0) only. Every texture must have exactly one sampler associated with it.
    // Samplers are defined by adding a SAMPLER RafxShaderResource with a valid gl_name. The
    // gl_sampler_name specified here will reference that sampler. While the GLSL code will not have
    // a sampler object, rafx API will act as though there is a sampler object. It can be set as if
    // it was a normal descriptor in a descriptor set. (Rafx shader processor can produce this
    // metadata automatically)
    pub gles_sampler_name: Option<String>,

    // Required for GL ES 2.0 only, every field within a uniform must be specified with the byte
    // offset. This includes elements within arrays. (Rafx shader processor can produce rust structs
    // and the necessary metadata automatically.)
    pub gles2_uniform_members: Vec<RafxGlUniformMember>,
}

impl Default for RafxShaderResource {
    fn default() -> Self {
        RafxShaderResource {
            resource_type: Default::default(),
            set_index: u32::MAX,
            binding: u32::MAX,
            element_count: 0,
            size_in_bytes: 0,
            used_in_shader_stages: Default::default(),
            name: None,
            dx12_reg: None,
            dx12_space: None,
            gles_name: None,
            gles_sampler_name: None,
            gles2_uniform_members: Vec::default(),
        }
    }
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

            if self.set_index != u32::MAX {
                Err(format!(
                    "binding (set={:?} binding={:?} name={:?} type={:?}) has set_index != u32::MAX",
                    self.set_index, self.binding, self.name, self.resource_type
                ))?;
            }

            if self.binding != u32::MAX {
                Err(format!(
                    "binding (set={:?} binding={:?} name={:?} type={:?}) has binding != u32::MAX",
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

            if self.set_index == u32::MAX {
                Err(format!(
                    "binding (set={:?} binding={:?} name={:?} type={:?}) has binding == u32::MAX",
                    self.set_index, self.binding, self.name, self.resource_type
                ))?;
            }

            if self.binding == u32::MAX {
                Err(format!(
                    "binding (set={:?} binding={:?} name={:?} type={:?}) has binding == u32::MAX",
                    self.set_index, self.binding, self.name, self.resource_type
                ))?;
            }

            if self.set_index as usize >= MAX_DESCRIPTOR_SET_LAYOUTS {
                Err(format!(
                    "Descriptor (set={:?} binding={:?}) named {:?} has a set index >= 4. This is not supported",
                    self.set_index, self.binding, self.name,
                ))?;
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
                self.binding,
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

        if self.gles2_uniform_members != other.gles2_uniform_members {
            Err(format!(
                "Pass is using shaders in different stages with different gl_uniform_members (set={} binding={})",
                self.set_index, self.binding
            ))?;
        }

        if self.gles_name != other.gles_name {
            Err(format!(
                "Pass is using shaders in different stages with different gles2_name (set={} binding={})",
                self.set_index, self.binding
            ))?;
        }

        if self.dx12_reg != other.dx12_reg {
            Err(format!(
                "Pass is using shaders in different stages with different dx12_reg (set={} binding={})",
                self.set_index, self.binding
            ))?;
        }

        if self.dx12_space != other.dx12_space {
            Err(format!(
                "Pass is using shaders in different stages with different dx12_space (set={} binding={})",
                self.set_index, self.binding
            ))?;
        }

        if self.gles_sampler_name.is_some()
            && other.gles_sampler_name.is_some()
            && self.gles_sampler_name != other.gles_sampler_name
        {
            Err(format!(
                "Pass is using shaders in different stages with different non-None gles2_sampler_name (set={} binding={})",
                self.set_index, self.binding
            ))?;
        }

        Ok(())
    }
}

/// Reflection data for a single shader stage
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct RafxShaderStageReflection {
    // For now, this doesn't do anything, so commented out
    //pub vertex_inputs: Vec<RafxVertexInput>,
    pub shader_stage: RafxShaderStageFlags,
    pub resources: Vec<RafxShaderResource>,
    pub compute_threads_per_group: Option<[u32; 3]>,
    pub entry_point_name: String,
    // Right now we will infer mappings based on spirv_cross default behavior, but likely will want
    // to allow providing them explicitly. This isn't implemented yet
    //pub binding_arg_buffer_mappings: FnvHashMap<(u32, u32), u32>
}

/// Reflection data for a pipeline, created by merging shader stage reflection data
#[derive(Debug)]
pub struct RafxPipelineReflection {
    pub shader_stages: RafxShaderStageFlags,
    pub resources: Vec<RafxShaderResource>,
    pub compute_threads_per_group: Option<[u32; 3]>,
}

impl RafxPipelineReflection {
    pub fn from_stages(stages: &[RafxShaderStageDef]) -> RafxResult<RafxPipelineReflection> {
        let mut unmerged_resources = Vec::default();
        for stage in stages {
            assert!(!stage.reflection.shader_stage.is_empty());
            for resource in &stage.reflection.resources {
                // The provided resource MAY (but does not need to) have the shader stage flag set.
                // (Leaving it default empty is fine). It will automatically be set here.
                if !(resource.used_in_shader_stages - stage.reflection.shader_stage).is_empty() {
                    let message = format!(
                        "A resource in shader stage {:?} has other stages {:?} set",
                        stage.reflection.shader_stage,
                        resource.used_in_shader_stages - stage.reflection.shader_stage
                    );
                    log::error!("{}", message);
                    Err(message)?;
                }

                let mut resource = resource.clone();
                resource.used_in_shader_stages |= stage.reflection.shader_stage;
                unmerged_resources.push(resource);
            }
        }

        let mut compute_threads_per_group = None;
        for stage in stages {
            if stage
                .reflection
                .shader_stage
                .intersects(RafxShaderStageFlags::COMPUTE)
            {
                compute_threads_per_group = stage.reflection.compute_threads_per_group;
            }
        }

        log::trace!("Create RafxPipelineReflection from stages");
        let mut all_shader_stages = RafxShaderStageFlags::empty();
        for stage in stages {
            if all_shader_stages.intersects(stage.reflection.shader_stage) {
                Err(format!(
                    "Duplicate shader stage ({}) found when creating RafxPipelineReflection",
                    (all_shader_stages & stage.reflection.shader_stage).bits()
                ))?;
            }

            all_shader_stages |= stage.reflection.shader_stage;
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
                if existing_resource.gles_sampler_name.is_none() {
                    existing_resource.gles_sampler_name = resource.gles_sampler_name.clone();
                }
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
            compute_threads_per_group,
            resources,
        })
    }
}
