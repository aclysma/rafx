use std::ffi::CString;
use crate::gl::{gles20, ProgramId, GlContext, LocationId};
use crate::{RafxResult, RafxShader};
use fnv::FnvHashMap;
use std::ops::Range;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct UniformIndex(pub u32);

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FieldIndex(pub u32);

#[derive(Debug)]
pub struct UniformInfo {
    name: CString,
    first_field_index: FieldIndex,
    field_count: u32,
}

#[derive(Debug)]
pub struct UniformFieldInfo {
    size: u32,
    ty: gles20::types::GLenum,
    field_index: FieldIndex,
    offset: u32,
}

#[derive(Debug)]
pub struct UniformReflectionData {
    program_ids: Vec<ProgramId>,
    uniforms: Vec<UniformInfo>,
    fields: Vec<UniformFieldInfo>,
    locations: Vec<Option<LocationId>>,
}

impl UniformReflectionData {
    pub fn new(
        gl_context: &GlContext,
        program_ids: Vec<ProgramId>,
        shaders: &[RafxShader],
    ) -> RafxResult<UniformReflectionData> {
        #[derive(Debug)]
        struct SizeTypeName {
            size: u32,
            ty: gles20::types::GLenum,
            name: CString,
        }

        let mut all_uniform_member_offsets = FnvHashMap::<String, u32>::default();
        for shader in shaders {
            for stage in shader.gl_shader().unwrap().stages() {
                for resource in &stage.reflection.resources {
                    for uniform_member in &resource.gl_uniform_members {
                        let old = all_uniform_member_offsets.insert(uniform_member.name.clone(), uniform_member.offset);
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

        for &program_id in &program_ids {
            let active_uniform_count = gl_context.gl_get_programiv(program_id, gles20::ACTIVE_UNIFORMS)? as u32;
            let max_name_length_hint = gl_context.get_active_uniform_max_name_length_hint(program_id)?;

            // Merges all the uniforms from the program into uniform_lookup and field_lookup
            for i in 0..active_uniform_count {
                let mut uniform_info = unsafe {
                    gl_context.gl_get_active_uniform(
                        program_id,
                        i,
                        &max_name_length_hint,
                    )
                }?;

                gl_context.check_for_error()?;

                // Find the first part of the name (everything up to but not including the first dot)
                let first_split = uniform_info.name
                    .to_bytes()
                    .iter()
                    .position(|x| *x == '.' as u8)
                    .unwrap_or(uniform_info.name.to_bytes().len());

                let uniform_name = CString::new(&uniform_info.name.to_bytes()[0..first_split]).unwrap();

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
                        name: full_name
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
        // Indexed by (field_index * program_count) + program_index
        let mut locations = Vec::<Option<LocationId>>::default();

        for (uniform_name, uniform_fields) in uniform_lookup {
            let uniform_info = UniformInfo {
                name: uniform_name,
                field_count: uniform_fields.len() as u32,
                first_field_index: FieldIndex(fields.len() as u32)
            };

            uniforms.push(uniform_info);

            for size_type_name in uniform_fields {
                let name_as_str = size_type_name.name.to_string_lossy();
                let offset = *all_uniform_member_offsets
                    .get(&*name_as_str)
                    .ok_or_else(|| format!("Could not find uniform member {} in the metadata for any shader stage", name_as_str))?;

                let field_info = UniformFieldInfo {
                    size: size_type_name.size,
                    ty: size_type_name.ty,
                    field_index: FieldIndex(fields.len() as u32),
                    offset
                };

                fields.push(field_info);

                for &program_id in &program_ids {
                    unsafe {
                        let location = gl_context.gl_get_uniform_location(program_id, &size_type_name.name)?;
                        locations.push(location);
                        //println!("{} {}", location, size_type_name.name.to_string_lossy());
                    }
                }
            }
        }

        Ok(UniformReflectionData {
            program_ids,
            uniforms,
            fields,
            locations
        })
    }

    pub fn uniform_index(&self, name: &CString) -> Option<u32> {
        self.uniforms
            .iter()
            .position(|x| x.name == *name)
            .map(|x| x as u32)
    }

    pub fn field_range(&self, uniform_index: UniformIndex) -> Range<usize> {
        let uniform = &self.uniforms[uniform_index.0 as usize];
        let first = uniform.first_field_index.0 as usize;
        let last = uniform.first_field_index.0 as usize + uniform.field_count as usize;
        first..last
    }

    pub fn fields(&self, uniform_index: UniformIndex) -> &[UniformFieldInfo] {
        let field_range = self.field_range(uniform_index);
        return &self.fields[field_range];
    }

    fn program_index(&self, program_id: ProgramId) -> Option<u32> {
        self.program_ids
            .iter()
            .position(|x| *x == program_id)
            .map(|x| x as u32)
    }

    pub fn location(&self, program_id: ProgramId, field_index: FieldIndex) -> Option<LocationId> {
        let program_index = self.program_index(program_id)?;

        self.locations[(field_index.0 as usize * self.program_ids.len()) + program_index as usize].clone()
    }
}
