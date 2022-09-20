use crate::parse_declarations::{ParseDeclarationsResult, ParseFieldResult};
use fnv::FnvHashMap;
use std::sync::Arc;

// https://graphics.stanford.edu/~seander/bithacks.html#RoundUpPowerOf2
fn next_power_of_2(mut v: usize) -> usize {
    v -= 1;
    v |= v >> 1;
    v |= v >> 2;
    v |= v >> 4;
    v |= v >> 8;
    v |= v >> 16;
    v |= v >> 32;
    v += 1;
    v
}

#[derive(Debug)]
pub(crate) enum StructOrBinding {
    Struct(usize),
    Binding(usize),
}

#[derive(Debug)]
pub(crate) struct TypeAlignmentInfo {
    pub(crate) rust_type: String,
    pub(crate) size: usize,
    pub(crate) align: usize,
    pub(crate) std140_alignment: usize, // for structs/array elements, round up to multiple of 16
    pub(crate) std430_alignment: usize,
}

#[derive(Debug)]
pub(crate) struct UserType {
    pub(crate) _struct_or_binding: StructOrBinding,
    pub(crate) type_name: String,
    pub(crate) fields: Arc<Vec<ParseFieldResult>>,
    //export_name: Option<String>,
    pub(crate) export_uniform_layout: bool,
    pub(crate) export_push_constant_layout: bool,
    pub(crate) export_buffer_layout: bool,
}

pub(crate) fn recursive_modify_user_type<F: Fn(&mut UserType) -> bool>(
    user_types: &mut FnvHashMap<String, UserType>,
    type_name: &str,
    f: &F,
) {
    let user_type = user_types.get_mut(type_name);
    let recurse = if let Some(user_type) = user_type {
        (f)(user_type)
    } else {
        // for now skip types we don't recognize
        return;
    };

    if recurse {
        if let Some(fields) = user_types.get(type_name).map(|x| x.fields.clone()) {
            for field in &*fields {
                recursive_modify_user_type(user_types, &field.type_name, f);
            }
        }
    }
}

pub(crate) fn create_user_type_lookup(
    parsed_declarations: &ParseDeclarationsResult
) -> Result<FnvHashMap<String, UserType>, String> {
    let mut user_types = FnvHashMap::<String, UserType>::default();

    //
    // Populate user types from structs
    //
    for (index, s) in parsed_declarations.structs.iter().enumerate() {
        //let export_name = s.annotations.export.as_ref().map(|x| x.0.clone());
        let old = user_types.insert(
            s.parsed.type_name.clone(),
            UserType {
                _struct_or_binding: StructOrBinding::Struct(index),
                type_name: s.parsed.type_name.clone(),
                fields: s.parsed.fields.clone(),
                //export_name,
                export_uniform_layout: false,
                export_push_constant_layout: false,
                export_buffer_layout: false,
            },
        );

        if old.is_some() {
            return Err(format!(
                "Duplicate user-defined struct type {}",
                s.parsed.type_name
            ));
        }
    }

    //
    // Populate user types from bindings
    //
    for (index, b) in parsed_declarations.bindings.iter().enumerate() {
        if let Some(fields) = &b.parsed.fields {
            //let struct_name_postfix = determine_binding_type(b)?.struct_name_postfix();
            //let struct_name_postfix = "";
            //let export_name = b.annotations.export.as_ref().map(|x| format!("{}{}", x.0, struct_name_postfix));
            //let adjusted_type_name = format!("{}{}", b.parsed.type_name, struct_name_postfix);
            //let export_name = b.annotations.export.as_ref().map(|x| x.0.clone());

            let old = user_types.insert(
                b.parsed.type_name.clone(),
                UserType {
                    _struct_or_binding: StructOrBinding::Binding(index),
                    type_name: b.parsed.type_name.clone(),
                    fields: fields.clone(),
                    //export_name,
                    export_uniform_layout: false,
                    export_push_constant_layout: false,
                    export_buffer_layout: false,
                },
            );

            if old.is_some() {
                return Err(format!(
                    "Duplicate user-defined binding type {}",
                    b.parsed.type_name
                ));
            }
        }
    }

    Ok(user_types)
}

fn add_type_alignment_info<T>(
    type_alignment_infos: &mut FnvHashMap<String, TypeAlignmentInfo>,
    type_name: &str,
    rust_type: &str,
) {
    let align = std::mem::align_of::<T>();
    let size = std::mem::size_of::<T>();

    let type_alignment_info = TypeAlignmentInfo {
        rust_type: rust_type.to_string(),
        size,
        align,
        // As far as I can tell, alignment is always 4, 8, or 16
        std140_alignment: next_power_of_2(size.min(16).max(4)),
        std430_alignment: next_power_of_2(size.min(16).max(4)),
    };
    log::trace!("built in type: {:?}", type_alignment_info);

    let old = type_alignment_infos.insert(type_name.to_string(), type_alignment_info);
    assert!(old.is_none());
}

#[rustfmt::skip]
pub(crate) fn create_builtin_type_lookup() -> FnvHashMap<String, TypeAlignmentInfo> {
    let mut builtin_types = FnvHashMap::<String, TypeAlignmentInfo>::default();
    add_type_alignment_info::<u32>(&mut builtin_types, "int", "i32");
    add_type_alignment_info::<u32>(&mut builtin_types, "uint", "u32");
    // treating a boolean as a u32 is the most straightworward solution
    add_type_alignment_info::<u32>(&mut builtin_types, "bool", "u32");
    add_type_alignment_info::<f32>(&mut builtin_types, "float", "f32");
    add_type_alignment_info::<[f32; 2]>(&mut builtin_types, "vec2", "[f32; 2]");
    add_type_alignment_info::<[f32; 3]>(&mut builtin_types, "vec3", "[f32; 3]");
    add_type_alignment_info::<[f32; 4]>(&mut builtin_types, "vec4", "[f32; 4]");
    add_type_alignment_info::<[[f32; 4]; 4]>(&mut builtin_types, "mat4", "[[f32; 4]; 4]");
    add_type_alignment_info::<[u32; 4]>(&mut builtin_types, "uvec4", "[u32; 4]");
    builtin_types
}

// Memory layouts we have to deal with (C = repr(C))
#[derive(Copy, Clone, Debug)]
pub enum MemoryLayout {
    Std140,
    Std430,
    C,
}

#[derive(Debug)]
pub struct StructMember {
    pub name: String,
    pub ty: String,
    pub size: usize,
    pub offset: usize,
    pub align: usize,
    pub default_value: String,
}

#[derive(Debug)]
pub struct GenerateStructResult {
    pub name: String,
    pub size: usize,
    pub align: usize,
    pub members: Vec<StructMember>,
}

pub(crate) fn generate_struct(
    builtin_types: &FnvHashMap<String, TypeAlignmentInfo>,
    user_types: &FnvHashMap<String, UserType>,
    type_name: &str,
    user_type: &UserType,
    layout: MemoryLayout,
) -> Result<GenerateStructResult, String> {
    //println!("Generate struct {}", type_name);

    let mut members = Vec::default();

    let mut pad_var_count = 0;

    let struct_name = get_rust_type_name_non_array(builtin_types, user_types, &type_name, layout)?;

    let mut gpu_offset = 0;
    let mut rust_offset = 0;
    for f in &*user_type.fields {
        //
        // Determine the alignment and size of this type using GPU layout
        //
        log::trace!("  get gpu required offset");
        let gpu_alignment = determine_alignment(
            builtin_types,
            user_types,
            &f.type_name,
            &f.array_sizes,
            layout,
        )?;

        log::trace!("    offset: {} align to {}", gpu_offset, gpu_alignment);
        gpu_offset = align_offset(gpu_offset, gpu_alignment);
        let maybe_gpu_size = determine_size(
            builtin_types,
            user_types,
            &f.type_name,
            &f.array_sizes,
            gpu_offset,
            gpu_offset,
            &f.type_name,
            layout,
        )?;
        if maybe_gpu_size.is_none() {
            return Err("Variable sized struct found, export is not supported as these can't be represented in rust")?;
        }
        let gpu_size = maybe_gpu_size.unwrap() - gpu_offset;

        //
        // Determine the alignment of this type in rust
        //
        log::trace!("  get rust required offset");
        let rust_alignment = determine_alignment(
            builtin_types,
            user_types,
            &f.type_name,
            &f.array_sizes,
            MemoryLayout::C,
        )?;

        log::trace!("    offset: {} align to {}", rust_offset, rust_alignment);
        let rust_required_offset = align_offset(rust_offset, rust_alignment);

        //
        // If there is too much padding in rust (not expected to happen), fail
        //
        if rust_required_offset > gpu_offset {
            let required_padding = rust_required_offset - gpu_offset;
            return Err(format!(
                "Field {}::{} ({}{}) requires {} bytes of padding in front of it. (The GPU memory layout has less padding that rust). Previous field ended at byte offset: {}",
                type_name,
                f.field_name,
                f.type_name,
                format_array_sizes(&f.array_sizes),
                required_padding,
                gpu_offset
            ));
        }

        //
        // If there is not enough padding in rust, add some
        //
        if rust_required_offset < gpu_offset {
            log::trace!(
                "Field {}::{} ({}{}) requires {} bytes of padding in front of it. (The GPU memory layout has more padding than rust). Previous field ended at byte offset: {}",
                type_name,
                f.field_name,
                f.type_name,
                format_array_sizes(&f.array_sizes),
                gpu_offset - rust_required_offset,
                rust_offset
            );

            let required_padding = gpu_offset - rust_required_offset;
            let struct_member = StructMember {
                name: format!("_padding{}", pad_var_count),
                ty: format!("[u8; {}]", required_padding),
                size: required_padding,
                align: 1,
                offset: rust_offset,
                default_value: format!("[u8::default(); {}]", required_padding),
            };
            log::trace!("member: {:?}", struct_member);
            members.push(struct_member);

            // Move the rust offset forward
            pad_var_count += 1;
            rust_offset += required_padding;
            log::trace!(
                "RUST: advance by {} bytes to offset {} (due to padding)",
                required_padding,
                rust_offset
            );
        }

        //
        // Determine the size of this var in rust. If it's a native type, the size is known ahead
        // of time and may be less than the gpu size. If it's a user-defined type, then we assume
        // padding is being inserted where necessary to ensure the sizes are the same, or that we
        // will flag an error if this is not possible.
        //
        assert_eq!(rust_offset, gpu_offset);
        let maybe_rust_size = determine_size_of_member_in_rust(
            builtin_types,
            user_types,
            &f.type_name,
            &f.array_sizes,
            rust_offset,
            rust_offset,
            &f.type_name,
            layout,
        )?;
        if maybe_rust_size.is_none() {
            return Err("Variable sized struct found, export is not supported as these can't be represented in rust")?;
        }
        let rust_size = maybe_rust_size.unwrap() - rust_offset;
        assert!(rust_size <= gpu_size);

        // It would be difficult to natively support this and make it ergonomic to use. Probably best to just avoid it.
        // In general, make sure array elements can align on 16-byte boundaries
        if !f.array_sizes.is_empty() && rust_size != gpu_size {
            return Err(format!(
                "Field {}::{} ({}{}) is an array of elements where the element type's GPU memory layout ({} size in bytes) does not match rust's memory layout ({} size in bytes). This is not currently supported. Try wrapping in a struct.",
                type_name,
                f.field_name,
                f.type_name,
                format_array_sizes(&f.array_sizes),
                gpu_size,
                rust_size
            ));
        }

        //
        // Add the member to the struct
        //
        let rust_type_name =
            get_rust_type_name_non_array(builtin_types, user_types, &f.type_name, layout)?;

        //println!("rust size: {}", rust_size);
        let struct_member = StructMember {
            name: f.field_name.clone(),
            ty: wrap_in_array(&rust_type_name, &f.array_sizes),
            size: rust_size,
            align: rust_alignment,
            offset: rust_offset,
            default_value: wrap_in_array(
                &format!("<{}>::default()", &rust_type_name),
                &f.array_sizes,
            ),
        };
        log::trace!("member: {:?}", struct_member);
        members.push(struct_member);

        // Move the rust and gpu offsets forward
        rust_offset += rust_size;
        gpu_offset += gpu_size;
        log::trace!(
            "RUST: advance by {} bytes to offset {}",
            rust_size,
            rust_offset
        );
        log::trace!(
            "GPU: advance by {} bytes to offset {}",
            gpu_size,
            gpu_offset
        );
    }

    //
    // Check how large this type is supposed to be in total. We will add padding on the end of the
    // struct to ensure that the rust type's size matches the size used in the gpu layout
    //
    let maybe_full_gpu_size = determine_size(
        builtin_types,
        user_types,
        &type_name,
        &[],
        0,
        0,
        &type_name,
        layout,
    )?;

    if maybe_full_gpu_size.is_none() {
        return Err("Variable sized struct found, export is not supported as these can't be represented in rust")?;
    }
    let full_gpu_size = maybe_full_gpu_size.unwrap();

    assert!(rust_offset <= full_gpu_size);
    if rust_offset < full_gpu_size {
        let required_padding = full_gpu_size - rust_offset;
        let struct_member = StructMember {
            name: format!("_padding{}", pad_var_count),
            ty: format!("[u8; {}]", required_padding),
            size: required_padding,
            align: 1,
            offset: rust_offset,
            default_value: format!("[u8::default(); {}]", required_padding),
        };
        log::trace!("member: {:?}", struct_member);
        members.push(struct_member);
    }

    let struct_align = determine_alignment_c(builtin_types, user_types, &type_name, &[])?;

    Ok(GenerateStructResult {
        name: struct_name,
        size: full_gpu_size,
        align: struct_align,
        members,
    })
}

pub(crate) fn wrap_in_array(
    inner: &str,
    array_sizes: &[usize],
) -> String {
    let mut wrapped = inner.to_string();
    for array_size in array_sizes.iter().rev() {
        wrapped = format!("[{}; {}]", wrapped, array_size);
    }

    wrapped
}

pub(crate) fn get_rust_type_name_non_array(
    builtin_types: &FnvHashMap<String, TypeAlignmentInfo>,
    user_types: &FnvHashMap<String, UserType>,
    name: &str,
    layout: MemoryLayout,
) -> Result<String, String> {
    let type_name = if let Some(builtin_type) = builtin_types.get(name) {
        builtin_type.rust_type.clone()
    } else if let Some(user_type) = user_types.get(name) {
        format!("{}{:?}", user_type.type_name.clone(), layout)
    } else {
        return Err(format!("Could not find type {}. Is this a built in type that needs to be added to create_builtin_type_lookup()?", name));
    };

    Ok(type_name)
}

pub(crate) fn format_array_sizes(sizes: &[usize]) -> String {
    let mut s = String::default();
    for size in sizes {
        s += &format!("[{}]", size);
    }

    s
}

fn align_offset(
    offset: usize,
    alignment: usize,
) -> usize {
    (offset + alignment - 1) / alignment * alignment
}

pub(crate) fn element_count(array_sizes: &[usize]) -> usize {
    if let Some(&last) = array_sizes.last() {
        if last == 0 {
            // It's a variable sized array
            return 0;
        }
    }

    let mut element_count = 1;
    for x in array_sizes {
        assert!(*x != 0);
        element_count *= x;
    }

    element_count
}

fn determine_size_of_member_in_rust(
    builtin_types: &FnvHashMap<String, TypeAlignmentInfo>,
    user_types: &FnvHashMap<String, UserType>,
    query_type: &str,
    array_sizes: &[usize],
    offset: usize,
    logging_offset: usize,
    logging_name: &str,
    gpu_layout: MemoryLayout,
) -> Result<Option<usize>, String> {
    let memory_layout = if builtin_types.contains_key(query_type) {
        MemoryLayout::C
    } else if user_types.contains_key(query_type) {
        gpu_layout
    } else {
        return Err(format!("Could not find type {}. Is this a built in type that needs to be added to create_builtin_type_lookup()?", query_type));
    };

    determine_size(
        builtin_types,
        user_types,
        query_type,
        array_sizes,
        offset,
        logging_offset,
        logging_name,
        memory_layout,
    )
}

fn determine_size(
    builtin_types: &FnvHashMap<String, TypeAlignmentInfo>,
    user_types: &FnvHashMap<String, UserType>,
    query_type: &str,
    array_sizes: &[usize],
    mut offset: usize,
    logging_offset: usize,
    logging_name: &str,
    layout: MemoryLayout,
) -> Result<Option<usize>, String> {
    // We only need to know how many elements we have
    let element_count = element_count(array_sizes);
    if element_count == 0 {
        // Variable sized struct returns None
        return Ok(None);
    }

    // Align this type (may be a struct, built-in, etc.
    // Caller should probably already align
    let alignment =
        determine_alignment(builtin_types, user_types, query_type, array_sizes, layout)?;
    assert_eq!(offset % alignment, 0);
    //offset = align_offset(offset, alignment);

    if let Some(builtin_type) = builtin_types.get(query_type) {
        //offset = align_offset(offset, alignment);

        log::trace!(
            "      {} +{} (size: {}) [{} elements of size {}, alignment: {}, name: {}]",
            query_type,
            logging_offset,
            element_count * builtin_type.size,
            element_count,
            builtin_type.size,
            alignment,
            logging_name
        );
        if array_sizes.is_empty() {
            // For example, a single vec3 is 16 byte aligned but only requires 12 bytes
            offset += builtin_type.size;
        } else {
            // Ensure every element is properly aligned
            // For example, a single vec3 is 16 byte aligned and in an array, every element is
            // 12 bytes for the vec + 4 padding
            let padded_size = align_offset(builtin_type.size, alignment);
            offset += padded_size * element_count;
        }

        Ok(Some(offset))
    } else if let Some(user_type) = user_types.get(query_type) {
        let mut offset_within_struct = 0;
        //println!("  process fields for {}", logging_name);
        for f in &*user_type.fields {
            // Align the member
            let field_alignment = determine_alignment(
                builtin_types,
                user_types,
                &f.type_name,
                &f.array_sizes,
                layout,
            )?;
            offset_within_struct = align_offset(offset_within_struct, field_alignment);

            let maybe_offset_within_struct = determine_size(
                builtin_types,
                user_types,
                &f.type_name,
                &f.array_sizes,
                offset_within_struct,
                offset + offset_within_struct,
                &f.field_name,
                layout,
            )?;

            if maybe_offset_within_struct.is_none() {
                return Ok(None);
            }

            offset_within_struct = maybe_offset_within_struct.unwrap();
        }

        let padded_size = align_offset(offset_within_struct, alignment);
        log::trace!(
            "        struct {} total size: {} [{} elements of size {} padded to {}]",
            logging_name,
            padded_size * element_count,
            element_count,
            offset_within_struct,
            padded_size
        );
        offset += padded_size * element_count;

        // // the base offset of the member following the sub-structure is rounded up to the next multiple of the base alignment of the structure
        // offset = (offset + alignment - 1) / alignment * alignment;
        Ok(Some(offset))
    } else {
        return Err(format!("Could not find type {}. Is this a built in type that needs to be added to create_builtin_type_lookup()?", query_type));
    }
}

fn determine_alignment(
    builtin_types: &FnvHashMap<String, TypeAlignmentInfo>,
    user_types: &FnvHashMap<String, UserType>,
    query_type: &str,
    array_sizes: &[usize],
    layout: MemoryLayout,
) -> Result<usize, String> {
    match layout {
        MemoryLayout::Std140 => {
            determine_alignment_std140(builtin_types, user_types, query_type, array_sizes)
        }
        MemoryLayout::Std430 => {
            determine_alignment_std430(builtin_types, user_types, query_type, array_sizes)
        }
        MemoryLayout::C => {
            determine_alignment_c(builtin_types, user_types, query_type, array_sizes)
        }
    }
}

//TODO: Do I need to generate structs for array elements that are not properly aligned?
fn determine_alignment_std140(
    builtin_types: &FnvHashMap<String, TypeAlignmentInfo>,
    user_types: &FnvHashMap<String, UserType>,
    query_type: &str,
    array_sizes: &[usize],
) -> Result<usize, String> {
    if let Some(builtin_type) = builtin_types.get(query_type) {
        if !array_sizes.is_empty() {
            // For std140, array element alignment is rounded up element to multiple of 16
            Ok(align_offset(builtin_type.std140_alignment, 16))
        } else {
            // Built-ins that are not array elements get normal alignment
            Ok(builtin_type.std140_alignment)
        }
    } else if let Some(user_type) = user_types.get(query_type) {
        let mut alignment = 16;
        for f in &*user_type.fields {
            let field_alignment = determine_alignment_std140(
                builtin_types,
                user_types,
                &f.type_name,
                &f.array_sizes,
            )?;

            // For std140, struct alignment is the max of its field's alignment requirements, rounded
            // up to 16
            //let field_alignment = (field_alignment + 15) / 16 * 16;
            alignment = alignment.max(field_alignment);
        }

        Ok(align_offset(alignment, 16))
    } else {
        return Err(format!("Could not find type {}. Is this a built in type that needs to be added to create_builtin_type_lookup()?", query_type));
    }
}

fn determine_alignment_std430(
    builtin_types: &FnvHashMap<String, TypeAlignmentInfo>,
    user_types: &FnvHashMap<String, UserType>,
    query_type: &str,
    _array_sizes: &[usize],
) -> Result<usize, String> {
    if let Some(builtin_type) = builtin_types.get(query_type) {
        Ok(builtin_type.std430_alignment)
    } else if let Some(user_type) = user_types.get(query_type) {
        let mut alignment = 4;
        for f in &*user_type.fields {
            let field_alignment = determine_alignment_std430(
                builtin_types,
                user_types,
                &f.type_name,
                &f.array_sizes,
            )?;
            alignment = alignment.max(field_alignment);
        }

        Ok(alignment)
    } else {
        return Err(format!("Could not find type {}. Is this a built in type that needs to be added to create_builtin_type_lookup()?", query_type));
    }
}

fn determine_alignment_c(
    builtin_types: &FnvHashMap<String, TypeAlignmentInfo>,
    user_types: &FnvHashMap<String, UserType>,
    query_type: &str,
    _array_sizes: &[usize],
) -> Result<usize, String> {
    if let Some(builtin_type) = builtin_types.get(query_type) {
        //Ok(next_power_of_2(builtin_type.size))
        Ok(builtin_type.align)
    } else if let Some(user_type) = user_types.get(query_type) {
        let mut alignment = 1;
        for f in &*user_type.fields {
            let field_alignment =
                determine_alignment_c(builtin_types, user_types, &f.type_name, &f.array_sizes)?;
            alignment = alignment.max(field_alignment);
        }

        Ok(alignment)
    } else {
        return Err(format!("Could not find type {}. Is this a built in type that needs to be added to create_builtin_type_lookup()?", query_type));
    }
}

pub(crate) fn verify_all_binding_layouts(
    builtin_types: &FnvHashMap<String, TypeAlignmentInfo>,
    user_types: &FnvHashMap<String, UserType>,
    reflect_data: &spirv_reflect::ShaderModule,
) -> Result<(), String> {
    for binding in reflect_data.enumerate_descriptor_bindings(None).unwrap() {
        use spirv_reflect::types::ReflectDescriptorType;
        let type_description = binding.type_description.as_ref().unwrap();
        match binding.descriptor_type {
            ReflectDescriptorType::UniformBuffer => {
                //println!("check uniform binding {}", binding.name);
                verify_layout(
                    builtin_types,
                    user_types,
                    &type_description.type_name,
                    &binding.block,
                    MemoryLayout::Std140,
                )?;
            }
            ReflectDescriptorType::StorageBuffer => {
                //println!("check buffer binding {}", binding.name);
                verify_layout(
                    builtin_types,
                    user_types,
                    &type_description.type_name,
                    &binding.block,
                    MemoryLayout::Std430,
                )?;
            }
            _ => {
                // No verification logic
            }
        }
    }

    // spirv-reflect seems to be producing wrong info for push constants
    /*
    for push_constant in reflect_data.enumerate_push_constant_blocks(None).unwrap() {
        //println!("check push constant binding {}", push_constant.name);
        let type_description = push_constant.type_description.as_ref().unwrap();
        println!("PUSH CONSTANT {:#?}", push_constant);
        verify_layout(
            builtin_types,
            user_types,
            &type_description.type_name,
            &push_constant,
            MemoryLayout::Std430,
        )?;
    }
    */

    Ok(())
}

fn verify_layout(
    builtin_types: &FnvHashMap<String, TypeAlignmentInfo>,
    user_types: &FnvHashMap<String, UserType>,
    type_name: &str,
    block: &spirv_reflect::types::ReflectBlockVariable,
    layout: MemoryLayout,
) -> Result<(), String> {
    //println!("{:?}", block);
    if !type_name.is_empty() {
        // println!(
        //     "check type {}",
        //     block.type_description.as_ref().unwrap().type_name
        // );

        let array_sizes: Vec<usize> = block.array.dims.iter().map(|x| *x as usize).collect();

        let size = determine_size(
            builtin_types,
            user_types,
            type_name,
            &array_sizes,
            0,
            0,
            type_name,
            layout,
        )?;

        // Skip checking for variables-sized structs
        if !size.is_none() {
            let size = size.unwrap();

            //NOTE: determine_size will, depending on type/array
            let alignment =
                determine_alignment(builtin_types, user_types, type_name, &array_sizes, layout)?;

            if block.padded_size != 0 {
                let padded_size = align_offset(block.size as usize, alignment);
                // The easy check, but it's 0 on storage buffers for some reason
                if size != padded_size {
                    fn print_block_members(
                        reflect_block_variable: &spirv_reflect::types::ReflectBlockVariable
                    ) {
                        for member in &reflect_block_variable.members {
                            log::info!("+{} (size {}) {}", member.offset, member.size, member.name);
                            print_block_members(&member);
                        }
                    }

                    print_block_members(block);

                    return Err(format!(
                        "Found a mismatch between logic and compiled spv alignments in type {} for layout {:?}. Logic size: {} SPV size is: {}",
                        type_name,
                        layout,
                        size,
                        block.size
                    ));
                }
            } else {
                // Alternative method, see comments and code here:
                // https://github.com/KhronosGroup/SPIRV-Cross/blob/e6f5ce6b8998f551f3400ad743b77be51bbe3019/spirv_cross.hpp#L246
                let mut size_from_reflection = block
                    .members
                    .last()
                    .map(|x| x.offset + x.padded_size)
                    .unwrap_or(0) as usize;

                // Sizes returned from determine_size will be padded
                size_from_reflection = align_offset(size_from_reflection, alignment);

                let element_count = element_count(&array_sizes);
                if size != size_from_reflection * element_count {
                    println!("alignment {}", alignment);
                    return Err(format!(
                        "Found a mismatch between logic and compiled spv alignments in type {} for layout {:?}. Logic size: {} SPV size is: {} ({} * {} elements, array sizes {:?})",
                        type_name,
                        layout,
                        size,
                        size_from_reflection * element_count,
                        size_from_reflection,
                        element_count,
                        array_sizes
                    ));
                }
            }
        }

        //println!("Layout from SPV: {:?}", block);
        //assert_eq!(size, block.padded_size as usize);
    }

    for block in &block.members {
        //println!("check uniform binding {}", block.name);
        verify_layout(
            builtin_types,
            user_types,
            &block.type_description.as_ref().unwrap().type_name,
            &block,
            layout,
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::parse_source::FileToProcess;
    use crate::shader_types::{create_builtin_type_lookup, create_user_type_lookup};

    fn verify_all_binding_layouts_in_test(
        reflect_data: spirv_reflect::ShaderModule,
        parsed_declarations: &ParseDeclarationsResult,
    ) {
        let user_types = create_user_type_lookup(parsed_declarations).unwrap();
        let builtin_types = create_builtin_type_lookup();

        verify_all_binding_layouts(&builtin_types, &user_types, &reflect_data).unwrap();
    }

    #[test]
    fn test_uniform_layout() {
        let shader_code = r#"
            #version 450

            struct PointLight {
                vec3 position_ws;
                vec3 position_vs;
                vec4 color;
                float range;
                float intensity;
            };

            struct DirectionalLight {
                vec3 direction_ws;
                vec3 direction_vs;
                vec4 color;
                float intensity;
            };

            struct SpotLight {
                vec3 position_ws;
                vec3 direction_ws;
                vec3 position_vs;
                vec3 direction_vs[2];
                vec4 color;
                float spotlight_half_angle;
                float range[5];
                float intensity[5][6];
            };

            // @[export]
            layout (set = 0, binding = 0) uniform PerViewData {
                vec4 ambient_light;
                uint point_light_count;
                uint directional_light_count;
                uint spot_light_count;
                PointLight point_lights[16];
                DirectionalLight directional_lights[16];
                SpotLight spot_lights[16];
            } per_frame_data;

            layout (location = 0) out vec4 out_color;
            void main() {
                out_color = vec4(per_frame_data.ambient_light);
            }
        "#;

        let (reflect_data, parsed_declarations) = compile_code_for_test(&shader_code);
        verify_all_binding_layouts_in_test(reflect_data, &parsed_declarations)
    }

    #[test]
    fn test_buffer_layout() {
        let shader_code = r#"
            #version 450

            struct PointLight {
                vec3 position_ws;
                vec3 position_vs;
                vec4 color;
                float range;
                float intensity;
            };

            struct DirectionalLight {
                vec3 direction_ws;
                vec3 direction_vs;
                vec4 color;
                float intensity;
            };

            struct SpotLight {
                vec3 position_ws;
                vec3 direction_ws;
                vec3 position_vs;
                vec3 direction_vs[2];
                vec4 color;
                float spotlight_half_angle;
                float range[5];
                float intensity[5][6];
            };

            // @[export]
            layout (set = 0, binding = 0) buffer PerViewData {
                vec4 ambient_light;
                uint point_light_count;
                uint directional_light_count;
                uint spot_light_count;
                PointLight point_lights[16];
                DirectionalLight directional_lights[16];
                SpotLight spot_lights[16];
            } per_frame_data;

            layout (location = 0) out vec4 out_color;
            void main() {
                out_color = vec4(per_frame_data.ambient_light);
            }
        "#;

        let (reflect_data, parsed_declarations) = compile_code_for_test(&shader_code);
        verify_all_binding_layouts_in_test(reflect_data, &parsed_declarations)
    }

    #[test]
    fn test_push_constant_layout() {
        let shader_code = r#"
            #version 450

            struct PointLight {
                vec3 position_ws;
                vec3 position_vs;
                vec4 color;
                float range;
                float intensity;
            };

            struct DirectionalLight {
                vec3 direction_ws;
                vec3 direction_vs;
                vec4 color;
                float intensity;
            };

            struct SpotLight {
                vec3 position_ws;
                vec3 direction_ws;
                vec3 position_vs;
                vec3 direction_vs[2];
                vec4 color;
                float spotlight_half_angle;
                float range[5];
                float intensity[5][6];
            };

            // @[export]
            layout (push_constant) uniform PerViewData {
                vec4 ambient_light;
                uint point_light_count;
                uint directional_light_count;
                uint spot_light_count;
                PointLight point_lights[16];
                DirectionalLight directional_lights[16];
                SpotLight spot_lights[16];
            } per_frame_data;

            layout (location = 0) out vec4 out_color;
            void main() {
                out_color = vec4(per_frame_data.ambient_light);
            }
        "#;

        let (reflect_data, parsed_declarations) = compile_code_for_test(&shader_code);
        verify_all_binding_layouts_in_test(reflect_data, &parsed_declarations)
    }

    // One reason for this test is to check that we can support the same structs used in different
    // layouts
    #[test]
    fn test_all_layout() {
        let shader_code = r#"
            #version 450

            struct PointLight {
                vec3 position_ws;
                vec3 position_vs;
                vec4 color;
                float range;
                float intensity;
            };

            struct DirectionalLight {
                vec3 direction_ws;
                vec3 direction_vs;
                vec4 color;
                float intensity;
            };

            struct SpotLight {
                vec3 position_ws;
                vec3 direction_ws;
                vec3 position_vs;
                vec3 direction_vs[2];
                vec4 color;
                float spotlight_half_angle;
                float range[5];
                float intensity[5][6];
            };

            // @[export]
            layout (set = 0, binding = 0) uniform PerViewDataUbo {
                vec4 ambient_light;
                uint point_light_count;
                uint directional_light_count;
                uint spot_light_count;
                PointLight point_lights[16];
                DirectionalLight directional_lights[16];
                SpotLight spot_lights[16];
            } per_frame_data_uniform;

            // @[export]
            layout (set = 0, binding = 1) buffer PerViewDataSbo {
                vec4 ambient_light;
                uint point_light_count;
                uint directional_light_count;
                uint spot_light_count;
                PointLight point_lights[16];
                DirectionalLight directional_lights[16];
                SpotLight spot_lights[16];
            } per_frame_data_buffer;

            // glsl required unique name for uniform blocks but reusing in uniform/buffers is allowed
            // we kind of have to support this anyways because the same struct can be used in all
            // 3 cases
            // @[export]
            layout (push_constant) uniform PerViewDataPC {
                vec4 ambient_light;
                uint point_light_count;
                uint directional_light_count;
                uint spot_light_count;
                PointLight point_lights[16];
                DirectionalLight directional_lights[16];
                SpotLight spot_lights[16];
            } per_frame_data_push_constant;

            layout (location = 0) out vec4 out_color;
            void main() {
                out_color = vec4(per_frame_data_uniform.ambient_light + per_frame_data_buffer.ambient_light + per_frame_data_push_constant.ambient_light);
            }
        "#;

        let (reflect_data, parsed_declarations) = compile_code_for_test(&shader_code);
        verify_all_binding_layouts_in_test(reflect_data, &parsed_declarations)
    }

    fn compile_code_for_test(
        shader_code: &str
    ) -> (
        spirv_reflect::ShaderModule,
        crate::parse_declarations::ParseDeclarationsResult,
    ) {
        // Compile it
        let compiler = shaderc::Compiler::new().unwrap();
        let result = compiler
            .compile_into_spirv(
                &shader_code,
                shaderc::ShaderKind::Fragment,
                "",
                "main",
                None,
            )
            .unwrap();

        // Scan SPV data
        let reflect_data = spirv_reflect::create_shader_module(result.as_binary_u8()).unwrap();

        // Parse it
        let file_to_process = FileToProcess {
            path: "".into(),
            include_type: crate::IncludeType::Relative,
            requested_from: "".into(),
            include_depth: 0,
        };

        let mut declarations = Vec::default();
        let mut included_files = Default::default();
        let code: Vec<char> = shader_code.chars().collect();
        crate::parse_source::parse_shader_source_text(
            &file_to_process,
            &mut declarations,
            &mut included_files,
            &code,
        )
        .unwrap();
        let parsed_declarations =
            crate::parse_declarations::parse_declarations(&declarations).unwrap();
        (reflect_data, parsed_declarations)
    }
}
