use crate::gl::{gles2_bindings, GlContext, ProgramId};
use crate::{RafxResult, RafxShader};
use fnv::FnvHashMap;
use std::ffi::CString;
use std::ops::Range;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct UniformIndex(pub u32);

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FieldIndex(pub u32);

#[derive(Debug)]
pub struct UniformInfo {
    pub(crate) name: CString,
    pub(crate) first_field_index: FieldIndex,
    pub(crate) field_count: u32,
}

#[derive(Debug)]
pub struct UniformFieldInfo {
    pub(crate) element_count: u32,
    pub(crate) ty: gles2_bindings::types::GLenum,
    pub(crate) field_index: FieldIndex,
    pub(crate) offset: u32,
    pub(crate) name: CString,
}

#[derive(Debug)]
pub struct UniformReflectionData {
    uniforms: Vec<UniformInfo>,
    fields: Vec<UniformFieldInfo>,
    uniform_name_lookup: FnvHashMap<String, UniformIndex>,
}

impl UniformReflectionData {
    pub fn new(
        gl_context: &GlContext,
        program_ids: &[ProgramId],
        shaders: &[RafxShader],
    ) -> RafxResult<UniformReflectionData> {
        #[derive(Debug)]
        struct SizeTypeName {
            // size is number of elements here, not bytes
            size: u32,
            ty: gles2_bindings::types::GLenum,
            name: CString,
        }

        let mut all_uniform_member_offsets = FnvHashMap::<String, u32>::default();
        for shader in shaders {
            for stage in shader.gles2_shader().unwrap().stages() {
                for resource in &stage.reflection.resources {
                    for uniform_member in &resource.gl_uniform_members {
                        let old = all_uniform_member_offsets
                            .insert(uniform_member.name.clone(), uniform_member.offset);
                        if let Some(offset) = old {
                            if offset != uniform_member.offset {
                                return Err(format!("Uniform member {} supplied multiple times with different offsets {} and {}", uniform_member.name, uniform_member.offset, offset))?;
                            }
                        }
                    }
                }
            }
        }

        // Temporary structures we use for merging uniforms/fields from multiple programs
        let mut uniform_lookup = FnvHashMap::<CString, Vec<SizeTypeName>>::default();
        let mut field_lookup = FnvHashMap::<CString, SizeTypeName>::default();

        for &program_id in program_ids {
            let active_uniform_count =
                gl_context.gl_get_programiv(program_id, gles2_bindings::ACTIVE_UNIFORMS)? as u32;
            let max_name_length_hint =
                gl_context.get_active_uniform_max_name_length_hint(program_id)?;

            // Merges all the uniforms from the program into uniform_lookup and field_lookup
            for i in 0..active_uniform_count {
                let uniform_info =
                    gl_context.gl_get_active_uniform(program_id, i, &max_name_length_hint)?;

                gl_context.check_for_error()?;

                // Find the first part of the name (everything up to but not including the first dot)
                let first_split = uniform_info
                    .name
                    .to_bytes()
                    .iter()
                    .position(|x| *x == '.' as u8)
                    .unwrap_or(uniform_info.name.to_bytes().len());

                let uniform_name =
                    CString::new(&uniform_info.name.to_bytes()[0..first_split]).unwrap();

                // Need to keep this so we can query GetUniformLocation later
                let full_name = uniform_info.name;
                let size = uniform_info.size;
                let ty = uniform_info.ty;

                if let Some(existing) = field_lookup.get_mut(&full_name) {
                    // verify the field metadata matches the other program's field metadata
                    if existing.size != size as u32 {
                        return Err(format!("Multiple programs with the same variable name {} but mismatching sizes of {} and {}", full_name.to_string_lossy(), existing.size, size))?;
                    } else if existing.ty != ty {
                        return Err(format!("Multiple programs with the same variable name {} but mismatching types of {} and {}", full_name.to_string_lossy(), existing.ty, ty))?;
                    }
                } else {
                    let field = SizeTypeName {
                        size: uniform_info.size as u32,
                        ty: uniform_info.ty,
                        name: full_name,
                    };

                    uniform_lookup.entry(uniform_name).or_default().push(field);
                }
            }
        }

        // This is the flattened data we will keep

        // Uniforms refer to a range of fields
        let mut uniforms = Vec::<UniformInfo>::default();
        // fields are stored grouped by uniform. This list is somewhat parallel with the locations list
        let mut fields = Vec::<UniformFieldInfo>::default();

        let mut uniform_name_lookup = FnvHashMap::<String, UniformIndex>::default();

        for (uniform_name, uniform_fields) in uniform_lookup {
            let uniform_name_str = uniform_name.clone().into_string().unwrap();
            let uniform_info = UniformInfo {
                name: uniform_name,
                field_count: uniform_fields.len() as u32,
                first_field_index: FieldIndex(fields.len() as u32),
            };

            let uniform_index = UniformIndex(uniforms.len() as u32);
            uniforms.push(uniform_info);
            let old = uniform_name_lookup.insert(uniform_name_str, uniform_index);
            assert!(old.is_none());

            for size_type_name in uniform_fields {
                let name_as_str = size_type_name.name.to_string_lossy();
                let offset = *all_uniform_member_offsets
                    .get(&*name_as_str)
                    .ok_or_else(|| {
                        format!(
                            "Could not find uniform member {} in the metadata for any shader stage",
                            name_as_str
                        )
                    })?;

                let field_info = UniformFieldInfo {
                    element_count: size_type_name.size,
                    ty: size_type_name.ty,
                    field_index: FieldIndex(fields.len() as u32),
                    offset,
                    name: size_type_name.name,
                };

                fields.push(field_info);
            }
        }

        Ok(UniformReflectionData {
            uniforms,
            fields,
            uniform_name_lookup,
        })
    }

    pub fn uniform_index(
        &self,
        name: &str,
    ) -> Option<UniformIndex> {
        self.uniform_name_lookup.get(name).cloned()
    }

    pub fn uniform_field_range(
        &self,
        uniform_index: UniformIndex,
    ) -> Range<usize> {
        let uniform = &self.uniforms[uniform_index.0 as usize];
        let first = uniform.first_field_index.0 as usize;
        let last = uniform.first_field_index.0 as usize + uniform.field_count as usize;
        first..last
    }

    pub fn uniform_fields(
        &self,
        uniform_index: UniformIndex,
    ) -> &[UniformFieldInfo] {
        let field_range = self.uniform_field_range(uniform_index);
        return &self.fields[field_range];
    }

    pub fn fields(&self) -> &[UniformFieldInfo] {
        &self.fields
    }
}
