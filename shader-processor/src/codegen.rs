use crate::parse_declarations::{
    BindingType, ParseDeclarationsResult, ParseFieldResult, ParsedBindingWithAnnotations,
};
use fnv::{FnvHashMap, FnvHashSet};
use renderer_assets::assets::reflect::ReflectedEntryPoint;
use renderer_resources::vk_description::DescriptorType;
use std::collections::BTreeMap;
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

// Structs can be used in one of these three ways. The usage will determine the memory layout
#[derive(Copy, Clone, Debug)]
enum StructBindingType {
    Uniform,
    Buffer,
    PushConstant,
}

// Determine the binding type of a struct based on parsed code
fn determine_binding_type(b: &ParsedBindingWithAnnotations) -> Result<StructBindingType, String> {
    if b.parsed.layout_parts.push_constant {
        Ok(StructBindingType::PushConstant)
    } else if b.parsed.binding_type == BindingType::Uniform {
        Ok(StructBindingType::Uniform)
    } else if b.parsed.binding_type == BindingType::Buffer {
        Ok(StructBindingType::Buffer)
    } else {
        Err("Unknown binding type".to_string())
    }
}

// Binding type determines memory layout that gets used
fn determine_memory_layout(binding_struct_type: StructBindingType) -> MemoryLayout {
    match binding_struct_type {
        StructBindingType::Uniform => MemoryLayout::Std140,
        StructBindingType::Buffer => MemoryLayout::Std430,
        StructBindingType::PushConstant => MemoryLayout::Std430,
    }
}

// Memory layouts we have to deal with (C = repr(C))
#[derive(Copy, Clone, Debug)]
enum MemoryLayout {
    Std140,
    Std430,
    C,
}

#[derive(Debug)]
enum StructOrBinding {
    Struct(usize),
    Binding(usize),
}

#[derive(Debug)]
struct TypeAlignmentInfo {
    rust_type: String,
    size: usize,
    align: usize,
    std140_alignment: usize, // for structs/array elements, round up to multiple of 16
    std430_alignment: usize,
}

#[derive(Debug)]
struct UserType {
    struct_or_binding: StructOrBinding,
    type_name: String,
    fields: Arc<Vec<ParseFieldResult>>,
    //export_name: Option<String>,
    export_uniform_layout: bool,
    export_push_constant_layout: bool,
    export_buffer_layout: bool,
}

fn recursive_modify_user_type<F: Fn(&mut UserType) -> bool>(
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

fn create_user_type_lookup(
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
                struct_or_binding: StructOrBinding::Struct(index),
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
                    struct_or_binding: StructOrBinding::Binding(index),
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
fn create_builtin_type_lookup() -> FnvHashMap<String, TypeAlignmentInfo> {
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
    builtin_types
}

pub(crate) fn generate_rust_code(
    parsed_declarations: &ParseDeclarationsResult,
    shader_module: &spirv_reflect::ShaderModule,
    reflected_entry_point: &ReflectedEntryPoint,
) -> Result<String, String> {
    //
    // Populate the user types map. Adding types in the map helps us detect duplicate type names
    // and quickly mark what layouts need to be exported (std140 - uniforms vs. std430 - push
    // constants/buffers)
    //
    // Structs and bindings can both declare new types, so gather data from both sources
    //
    let mut user_types = create_user_type_lookup(parsed_declarations)?;
    let builtin_types = create_builtin_type_lookup();

    verify_all_binding_layouts(&builtin_types, &user_types, shader_module)?;

    //
    // Any struct that's explicitly exported will produce all layouts
    //
    for s in &parsed_declarations.structs {
        if s.annotations.export.is_some() {
            recursive_modify_user_type(&mut user_types, &s.parsed.type_name, &|udt| {
                let already_marked = udt.export_uniform_layout
                    && udt.export_push_constant_layout
                    && udt.export_buffer_layout;
                udt.export_uniform_layout = true;
                udt.export_push_constant_layout = true;
                udt.export_buffer_layout = true;
                !already_marked
            });
        }
    }

    //
    // Bindings can either be std140 (uniform) or std430 (push constant/buffer). Depending on the
    // binding, enable export for just the type that we need
    //
    for b in &parsed_declarations.bindings {
        if b.annotations.export.is_some() {
            match determine_binding_type(b)? {
                StructBindingType::PushConstant => {
                    recursive_modify_user_type(&mut user_types, &b.parsed.type_name, &|udt| {
                        let already_marked = udt.export_push_constant_layout;
                        udt.export_push_constant_layout = true;
                        !already_marked
                    });
                }
                StructBindingType::Uniform => {
                    recursive_modify_user_type(&mut user_types, &b.parsed.type_name, &|udt| {
                        let already_marked = udt.export_uniform_layout;
                        udt.export_uniform_layout = true;
                        !already_marked
                    });
                }
                StructBindingType::Buffer => {
                    recursive_modify_user_type(&mut user_types, &b.parsed.type_name, &|udt| {
                        let already_marked = udt.export_buffer_layout;
                        udt.export_buffer_layout = true;
                        !already_marked
                    });
                }
            }
        }
    }

    generate_rust_file(
        &parsed_declarations,
        &builtin_types,
        &user_types,
        reflected_entry_point,
    )
}

fn generate_rust_file(
    parsed_declarations: &ParseDeclarationsResult,
    builtin_types: &FnvHashMap<String, TypeAlignmentInfo>,
    user_types: &FnvHashMap<String, UserType>,
    reflected_entry_point: &ReflectedEntryPoint,
) -> Result<String, String> {
    let mut rust_code = Vec::<String>::default();

    rust_header(&mut rust_code);

    let structs = rust_structs(&mut rust_code, builtin_types, user_types)?;

    rust_binding_constants(&mut rust_code, &parsed_declarations);

    rust_binding_wrappers(
        &mut rust_code,
        builtin_types,
        user_types,
        &parsed_declarations,
        reflected_entry_point,
    )?;

    rust_tests(&mut rust_code, &structs);

    let mut rust_code_str = String::default();
    for s in rust_code {
        rust_code_str += &s;
    }

    Ok(rust_code_str)
}

fn rust_header(rust_code: &mut Vec<String>) {
    rust_code.push("// This code is auto-generated by the shader processor.\n\n".to_string());

    rust_code.push("#[allow(unused_imports)]\n".to_string());
    rust_code.push("use renderer_resources::ash::prelude::VkResult;\n\n".to_string());
    rust_code.push("#[allow(unused_imports)]\n".to_string());
    rust_code.push("use renderer_resources::{ResourceArc, ImageViewResource, DynDescriptorSet, DescriptorSetAllocator, DescriptorSetInitializer, DescriptorSetArc};\n\n".to_string());
}

fn rust_structs(
    rust_code: &mut Vec<String>,
    builtin_types: &FnvHashMap<String, TypeAlignmentInfo>,
    user_types: &FnvHashMap<String, UserType>,
) -> Result<Vec<GenerateStructResult>, String> {
    let mut structs = Vec::default();
    for (type_name, user_type) in user_types {
        if user_type.export_uniform_layout {
            let s = generate_struct(
                &builtin_types,
                &user_types,
                type_name,
                user_type,
                MemoryLayout::Std140,
            )?;
            rust_code.push(s.generate_struct_code());
            rust_code.push(s.generate_struct_default_code());
            structs.push(s);
        }

        if user_type.export_uniform_layout {
            rust_code.push(format!(
                "pub type {} = {};\n\n",
                get_rust_type_name_alias(
                    builtin_types,
                    user_types,
                    &user_type.type_name,
                    &[],
                    StructBindingType::Uniform
                )?,
                get_rust_type_name(
                    builtin_types,
                    user_types,
                    &user_type.type_name,
                    MemoryLayout::Std140,
                    &[]
                )?
            ));
        }

        if user_type.export_push_constant_layout || user_type.export_buffer_layout {
            let s = generate_struct(
                &builtin_types,
                &user_types,
                type_name,
                user_type,
                MemoryLayout::Std430,
            )?;
            rust_code.push(s.generate_struct_code());
            structs.push(s);
        }

        if user_type.export_push_constant_layout {
            rust_code.push(format!(
                "pub type {} = {};\n\n",
                get_rust_type_name_alias(
                    builtin_types,
                    user_types,
                    &user_type.type_name,
                    &[],
                    StructBindingType::PushConstant
                )?,
                get_rust_type_name(
                    builtin_types,
                    user_types,
                    &user_type.type_name,
                    MemoryLayout::Std430,
                    &[]
                )?
            ));
        }
        if user_type.export_buffer_layout {
            rust_code.push(format!(
                "pub type {} = {};\n\n",
                get_rust_type_name_alias(
                    builtin_types,
                    user_types,
                    &user_type.type_name,
                    &[],
                    StructBindingType::Buffer
                )?,
                get_rust_type_name(
                    builtin_types,
                    user_types,
                    &user_type.type_name,
                    MemoryLayout::Std430,
                    &[]
                )?
            ));
        }
    }

    Ok(structs)
}

fn rust_binding_wrappers(
    rust_code: &mut Vec<String>,
    builtin_types: &FnvHashMap<String, TypeAlignmentInfo>,
    user_types: &FnvHashMap<String, UserType>,
    parsed_declarations: &ParseDeclarationsResult,
    reflected_entry_point: &ReflectedEntryPoint,
) -> Result<(), String> {
    let mut bindings_by_set =
        BTreeMap::<usize, BTreeMap<usize, &ParsedBindingWithAnnotations>>::default();
    for binding in &parsed_declarations.bindings {
        if let Some(set) = binding.parsed.layout_parts.set {
            if let Some(binding_index) = binding.parsed.layout_parts.binding {
                bindings_by_set
                    .entry(set)
                    .or_default()
                    .insert(binding_index, binding);
            }
        }
    }

    for (set_index, bindings) in bindings_by_set {
        let mut binding_wrapper_items = Vec::default();
        let mut binding_wrapper_struct_lifetimes = Vec::default();
        for (binding_index, binding) in bindings {
            create_binding_wrapper_binding_item(
                &mut binding_wrapper_items,
                &mut binding_wrapper_struct_lifetimes,
                user_types,
                builtin_types,
                reflected_entry_point,
                set_index,
                binding_index,
                binding,
            )?;
        }

        if binding_wrapper_items.is_empty() {
            continue;
        }

        let wrapper_struct_name = format!("DescriptorSet{}", set_index);

        let args_struct_name = format!("DescriptorSet{}Args", set_index);
        let wrapper_args_generic_params = if binding_wrapper_struct_lifetimes.is_empty() {
            String::default()
        } else {
            let mut set = FnvHashSet::default();
            let mut unique_lifetimes = Vec::default();

            for lifetime in binding_wrapper_struct_lifetimes {
                if set.insert(lifetime.clone()) {
                    unique_lifetimes.push(lifetime);
                }
            }

            format!("<{}>", unique_lifetimes.join(", "))
        };

        //
        // Args to create the descriptor set
        //
        rust_code.push(format!(
            "pub struct {}{} {{\n",
            args_struct_name, wrapper_args_generic_params
        ));
        for item in &binding_wrapper_items {
            rust_code.push(format!(
                "    pub {}: {},\n",
                item.binding_name, item.args_struct_member_type
            ));
        }
        rust_code.push("}\n\n".to_string());

        //
        // DescriptorSetInitializer trait impl
        //

        rust_code.push(format!(
            "impl<'a> DescriptorSetInitializer<'a> for {}{} {{\n",
            args_struct_name, wrapper_args_generic_params
        ));
        rust_code.push(format!("    type Output = {};\n\n", wrapper_struct_name));

        // create_dyn_descriptor_set
        rust_code.push("    fn create_dyn_descriptor_set(descriptor_set: DynDescriptorSet, args: Self) -> Self::Output {\n".to_string());
        rust_code.push(format!(
            "        let mut descriptor = {}(descriptor_set);\n",
            wrapper_struct_name
        ));
        rust_code.push("        descriptor.set_args(args);\n".to_string());
        rust_code.push("        descriptor\n".to_string());
        rust_code.push("    }\n\n".to_string());

        // create_descriptor_set
        rust_code.push("    fn create_descriptor_set(descriptor_set_allocator: &mut DescriptorSetAllocator, descriptor_set: DynDescriptorSet, args: Self) -> VkResult<DescriptorSetArc> {\n".to_string());
        rust_code.push(
            "        let mut descriptor = Self::create_dyn_descriptor_set(descriptor_set, args);\n"
                .to_string(),
        );
        rust_code.push("        descriptor.0.flush(descriptor_set_allocator)?;\n".to_string());
        rust_code.push("        Ok(descriptor.0.descriptor_set().clone())\n".to_string());
        rust_code.push("    }\n".to_string());

        rust_code.push("}\n\n".to_string());

        //
        // Wrapper struct
        //
        rust_code.push(format!(
            "pub struct {}(pub DynDescriptorSet);\n\n",
            wrapper_struct_name
        ));

        rust_code.push(format!("impl {} {{\n", wrapper_struct_name));

        //
        // set_args_static()
        //
        rust_code.push(format!(
            "    pub fn set_args_static(descriptor_set: &mut DynDescriptorSet, args: {}) {{\n",
            args_struct_name
        ));
        for item in &binding_wrapper_items {
            if item.descriptor_count == 1 {
                rust_code.push(format!(
                    "        descriptor_set.{}({}, args.{});\n",
                    item.setter_fn_name_single, item.binding_index_string, item.binding_name
                ));
            } else {
                rust_code.push(format!(
                    "        descriptor_set.{}({}, args.{});\n",
                    item.setter_fn_name_multi, item.binding_index_string, item.binding_name
                ));
            }
        }
        rust_code.push("    }\n\n".to_string());

        //
        // set_args()
        //
        rust_code.push(format!(
            "    pub fn set_args(&mut self, args: {}) {{\n",
            args_struct_name
        ));
        for item in &binding_wrapper_items {
            rust_code.push(format!(
                "        self.set_{}(args.{});\n",
                item.binding_name, item.binding_name
            ));
        }
        rust_code.push("    }\n\n".to_string());

        //
        // setters for individual bindings
        //
        //TODO: Make this support arrays
        for item in &binding_wrapper_items {
            if item.descriptor_count == 1 {
                //
                // Set the value
                //
                rust_code.push(format!(
                    "    pub fn set_{}(&mut self, {}: {}) {{\n",
                    item.binding_name, item.binding_name, item.set_element_param_type_single
                ));
                rust_code.push(format!(
                    "        self.0.{}({}, {});\n",
                    item.setter_fn_name_single, item.binding_index_string, item.binding_name
                ));
                rust_code.push("    }\n\n".to_string());
            } else if item.descriptor_count > 1 {
                //
                // Set all the values
                //
                rust_code.push(format!(
                    "    pub fn set_{}(&mut self, {}: {}) {{\n",
                    item.binding_name, item.binding_name, item.set_element_param_type_multi
                ));
                rust_code.push(format!(
                    "        self.0.{}({}, {});\n",
                    item.setter_fn_name_multi, item.binding_index_string, item.binding_name
                ));
                rust_code.push("    }\n\n".to_string());

                //
                // Set one of the values
                //
                rust_code.push(format!(
                    "    pub fn set_{}_element(&mut self, index: usize, element: {}) {{\n",
                    item.binding_name, item.set_element_param_type_single
                ));
                rust_code.push(format!(
                    "        self.0.{}({}, index, element);\n",
                    item.setter_fn_name_single, item.binding_index_string
                ));
                rust_code.push("    }\n\n".to_string());
            }
        }

        //
        // flush
        //
        rust_code.push("    pub fn flush(&mut self, descriptor_set_allocator: &mut DescriptorSetAllocator) -> VkResult<()> {\n".to_string());
        rust_code.push("        self.0.flush(descriptor_set_allocator)\n".to_string());
        rust_code.push("    }\n".to_string());

        rust_code.push("}\n\n".to_string());
    }

    Ok(())
}

struct BindingWrapperItem {
    binding_name: String,
    setter_fn_name_single: String,
    setter_fn_name_multi: String,
    args_struct_member_type: String,
    set_element_param_type_single: String,
    set_element_param_type_multi: String,
    binding_index_string: String,
    descriptor_count: u32,
}

fn create_binding_wrapper_binding_item(
    binding_wrapper_items: &mut Vec<BindingWrapperItem>,
    binding_wrapper_struct_lifetimes: &mut Vec<String>,
    user_types: &FnvHashMap<String, UserType>,
    builtin_types: &FnvHashMap<String, TypeAlignmentInfo>,
    reflected_entry_point: &ReflectedEntryPoint,
    set_index: usize,
    binding_index: usize,
    binding: &ParsedBindingWithAnnotations,
) -> Result<(), String> {
    // Don't generate member function for this binding if the type wasn't exported
    if !binding.annotations.export.is_some() {
        return Ok(());
    }

    // Find the binding in the reflection data
    let e = reflected_entry_point
        .descriptor_set_layouts
        .get(set_index)
        .ok_or_else(|| {
            format!(
                "Could not find descriptor set index {} in reflection data",
                set_index
            )
        })?
        .as_ref()
        .ok_or_else(|| {
            format!(
                "Could not find descriptor set index {} in reflection data",
                set_index
            )
        })?
        .bindings
        .iter()
        .find(|x| x.binding == binding_index as u32)
        .ok_or_else(|| {
            format!(
                "Could not find descriptor binding index {} in reflection data",
                binding_index
            )
        })?;

    use heck::SnakeCase;
    let binding_name = binding.parsed.instance_name.to_snake_case();
    let binding_index_string =
        format!("{} as u32", descriptor_constant_binding_index_name(binding));

    match e.descriptor_type {
        DescriptorType::Sampler => {
            if e.immutable_samplers.is_none() {
                // TODO: Generate a setter for samplers
            }
        }
        DescriptorType::CombinedImageSampler => {
            if e.immutable_samplers.is_none() {
                // TODO: Generate a setter for image and sampler?
            } else {
                // TODO: Generate a setter for image only?
            }
        }
        DescriptorType::SampledImage | DescriptorType::StorageImage => {
            if e.descriptor_count > 1 {
                binding_wrapper_items.push(BindingWrapperItem {
                    binding_name,
                    setter_fn_name_single: "set_image_at_index".to_string(),
                    setter_fn_name_multi: "set_images".to_string(),
                    args_struct_member_type: format!(
                        "&'a [Option<&'a ResourceArc<ImageViewResource>>; {}]",
                        e.descriptor_count
                    )
                    .to_string(),
                    set_element_param_type_single: "&ResourceArc<ImageViewResource>".to_string(),
                    set_element_param_type_multi: format!(
                        "&[Option<& ResourceArc<ImageViewResource>>; {}]",
                        e.descriptor_count
                    )
                    .to_string(),
                    binding_index_string,
                    descriptor_count: e.descriptor_count,
                });
            } else {
                binding_wrapper_items.push(BindingWrapperItem {
                    binding_name,
                    setter_fn_name_single: "set_image".to_string(),
                    setter_fn_name_multi: "set_images".to_string(),
                    args_struct_member_type: "&'a ResourceArc<ImageViewResource>".to_string(),
                    set_element_param_type_single: "&ResourceArc<ImageViewResource>".to_string(),
                    set_element_param_type_multi: "&[& ResourceArc<ImageViewResource>]".to_string(),
                    binding_index_string,
                    descriptor_count: e.descriptor_count,
                });
            }
            binding_wrapper_struct_lifetimes.push("'a".to_string());
        }
        DescriptorType::UniformBuffer | DescriptorType::StorageBuffer => {
            assert_eq!(e.descriptor_count, 1);
            let type_name = get_rust_type_name_alias(
                builtin_types,
                user_types,
                &binding.parsed.type_name,
                &binding.parsed.array_sizes,
                determine_binding_type(binding)?,
            )?;
            binding_wrapper_items.push(BindingWrapperItem {
                binding_name,
                setter_fn_name_single: "set_buffer_data".to_string(),
                setter_fn_name_multi: "set_buffer_data".to_string(),
                args_struct_member_type: format!("&'a {}", type_name),
                set_element_param_type_single: format!("&{}", type_name),
                set_element_param_type_multi: format!("&'[{}]", type_name),
                binding_index_string,
                descriptor_count: e.descriptor_count,
            });
            binding_wrapper_struct_lifetimes.push("'a".to_string());
        }
        // No support for these yet
        DescriptorType::UniformBufferDynamic => {}
        DescriptorType::StorageBufferDynamic => {}
        DescriptorType::UniformTexelBuffer => {}
        DescriptorType::StorageTexelBuffer => {}
        DescriptorType::InputAttachment => {}
    };

    Ok(())
}

fn descriptor_constant_set_index_name(binding: &ParsedBindingWithAnnotations) -> String {
    use heck::ShoutySnakeCase;
    format!(
        "{}_DESCRIPTOR_SET_INDEX",
        binding.parsed.instance_name.to_shouty_snake_case()
    )
}

fn descriptor_constant_binding_index_name(binding: &ParsedBindingWithAnnotations) -> String {
    use heck::ShoutySnakeCase;
    format!(
        "{}_DESCRIPTOR_BINDING_INDEX",
        binding.parsed.instance_name.to_shouty_snake_case()
    )
}

fn rust_binding_constants(
    rust_code: &mut Vec<String>,
    parsed_declarations: &ParseDeclarationsResult,
) {
    for binding in &parsed_declarations.bindings {
        if let Some(set_index) = binding.parsed.layout_parts.set {
            rust_code.push(format!(
                "pub const {}: usize = {};\n",
                descriptor_constant_set_index_name(binding),
                set_index
            ));
        }

        if let Some(binding_index) = binding.parsed.layout_parts.binding {
            rust_code.push(format!(
                "pub const {}: usize = {};\n",
                descriptor_constant_binding_index_name(binding),
                binding_index
            ));
        }
    }

    rust_code.push("\n".to_string());
}

fn rust_tests(
    rust_code: &mut Vec<String>,
    structs: &[GenerateStructResult],
) {
    if !structs.is_empty() {
        rust_code.push("#[cfg(test)]\nmod test {\n    use super::*;\n".to_string());
        for s in structs {
            rust_code.push(s.generate_struct_test_code());
        }
        rust_code.push("}\n".to_string());
    }
}

fn wrap_in_array(
    inner: &str,
    array_sizes: &[usize],
) -> String {
    let mut wrapped = inner.to_string();
    for array_size in array_sizes.iter().rev() {
        wrapped = format!("[{}; {}]", wrapped, array_size);
    }

    wrapped
}

fn get_rust_type_name_non_array(
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

fn get_rust_type_name(
    builtin_types: &FnvHashMap<String, TypeAlignmentInfo>,
    user_types: &FnvHashMap<String, UserType>,
    name: &str,
    layout: MemoryLayout,
    array_sizes: &[usize],
) -> Result<String, String> {
    let type_name = get_rust_type_name_non_array(builtin_types, user_types, name, layout)?;

    Ok(wrap_in_array(&type_name, array_sizes))
}

fn get_rust_type_name_alias(
    builtin_types: &FnvHashMap<String, TypeAlignmentInfo>,
    user_types: &FnvHashMap<String, UserType>,
    name: &str,
    array_sizes: &[usize],
    binding_struct_type: StructBindingType,
) -> Result<String, String> {
    let layout = determine_memory_layout(binding_struct_type);
    let alias_name = format!("{:?}", binding_struct_type);

    if builtin_types.contains_key(name) {
        get_rust_type_name(builtin_types, user_types, name, layout, array_sizes)
    } else if let Some(user_type) = user_types.get(name) {
        Ok(format!(
            "{}{}{}",
            user_type.type_name.clone(),
            alias_name,
            format_array_sizes(array_sizes)
        ))
    } else {
        Err(format!("Could not find type {}. Is this a built in type that needs to be added to create_builtin_type_lookup()?", name))
    }
}

fn format_array_sizes(sizes: &[usize]) -> String {
    let mut s = String::default();
    for size in sizes {
        s += &format!("[{}]", size);
    }

    s
}

#[derive(Debug)]
struct StructMember {
    name: String,
    ty: String,
    size: usize,
    offset: usize,
    align: usize,
    default_value: String,
}

#[derive(Debug)]
struct GenerateStructResult {
    name: String,
    size: usize,
    align: usize,
    members: Vec<StructMember>,
}

impl GenerateStructResult {
    fn generate_struct_code(&self) -> String {
        let mut result_string = String::default();
        result_string += &format!(
            "#[derive(Copy, Clone, Debug)]\n#[repr(C)]\npub struct {} {{\n",
            self.name
        );
        for m in &self.members {
            result_string += &format_member(&m.name, &m.ty, m.offset, m.size);
        }
        result_string += &format!("}} // {} bytes\n\n", self.size);
        result_string
    }

    fn generate_struct_default_code(&self) -> String {
        let mut result_string = String::default();
        result_string += &format!("impl Default for {} {{\n", self.name);
        result_string += &format!("    fn default() -> Self {{\n");
        result_string += &format!("        {} {{\n", self.name);
        for m in &self.members {
            //result_string += &format!("            {}: {}::default(),\n", &m.name, &m.ty);
            result_string += &format!("            {}: {},\n", &m.name, m.default_value);
        }
        result_string += &format!("        }}\n");
        result_string += &format!("    }}\n");
        result_string += &format!("}}\n\n");
        result_string
    }

    fn generate_struct_test_code(&self) -> String {
        use heck::SnakeCase;
        let mut result_string = String::default();
        result_string += &format!(
            "\n    #[test]\n    fn test_struct_{}() {{\n",
            self.name.to_snake_case()
        );
        result_string += &format!(
            "        assert_eq!(std::mem::size_of::<{}>(), {});\n",
            self.name, self.size
        );
        for m in &self.members {
            result_string += &format!(
                "        assert_eq!(std::mem::size_of::<{}>(), {});\n",
                m.ty, m.size
            );
            result_string += &format!(
                "        assert_eq!(std::mem::align_of::<{}>(), {});\n",
                m.ty, m.align
            );
            result_string += &format!(
                "        assert_eq!(memoffset::offset_of!({}, {}), {});\n",
                self.name, m.name, m.offset
            );
        }
        result_string += &format!("    }}\n");
        result_string
    }
}

fn format_member(
    name: &str,
    ty: &str,
    offset: usize,
    size: usize,
) -> String {
    let mut str = format!("    pub {}: {}, ", name, ty);
    let whitespace = 40_usize.saturating_sub(str.len());
    str += " ".repeat(whitespace).as_str();
    str += &format!("// +{} (size: {})\n", offset, size);
    str
}

fn generate_struct(
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
        let gpu_size = determine_size(
            builtin_types,
            user_types,
            &f.type_name,
            &f.array_sizes,
            gpu_offset,
            gpu_offset,
            &f.type_name,
            layout,
        )? - gpu_offset;

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
        let rust_size = determine_size_of_member_in_rust(
            builtin_types,
            user_types,
            &f.type_name,
            &f.array_sizes,
            rust_offset,
            rust_offset,
            &f.type_name,
            layout,
        )? - rust_offset;
        assert!(rust_size <= gpu_size);

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
    let full_gpu_size = determine_size(
        builtin_types,
        user_types,
        &type_name,
        &[],
        0,
        0,
        &type_name,
        layout,
    )?;

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

fn align_offset(
    offset: usize,
    alignment: usize,
) -> usize {
    (offset + alignment - 1) / alignment * alignment
}

fn element_count(array_sizes: &[usize]) -> usize {
    let mut element_count = 1;
    for x in array_sizes {
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
) -> Result<usize, String> {
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
) -> Result<usize, String> {
    // We only need to know how many elements we have
    let element_count = element_count(array_sizes);

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

        Ok(offset)
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

            offset_within_struct = determine_size(
                builtin_types,
                user_types,
                &f.type_name,
                &f.array_sizes,
                offset_within_struct,
                offset + offset_within_struct,
                &f.field_name,
                layout,
            )?;
        }

        let padded_size = align_offset(offset_within_struct, alignment);
        log::trace!(
            "        struct {} total size: {} [{} elements of size {}]",
            logging_name,
            padded_size * element_count,
            element_count,
            padded_size
        );
        offset += padded_size * element_count;

        // // the base offset of the member following the sub-structure is rounded up to the next multiple of the base alignment of the structure
        // offset = (offset + alignment - 1) / alignment * alignment;
        Ok(offset)
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

fn verify_all_binding_layouts(
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

    for push_constant in reflect_data.enumerate_push_constant_blocks(None).unwrap() {
        //println!("check push constant binding {}", push_constant.name);
        let type_description = push_constant.type_description.as_ref().unwrap();
        verify_layout(
            builtin_types,
            user_types,
            &type_description.type_name,
            &push_constant,
            MemoryLayout::Std430,
        )?;
    }

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

        if block.padded_size != 0 {
            // The easy check, but it's 0 on storage buffers for some reason
            if size != block.size as usize {
                println!("{:?}", block);

                fn print_block_members(
                    reflect_block_variable: &spirv_reflect::types::ReflectBlockVariable
                ) {
                    for member in &reflect_block_variable.members {
                        log::trace!("+{} (size {}) {}", member.offset, member.size, member.name);
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
            let size_from_reflection = block
                .members
                .last()
                .map(|x| x.offset + x.padded_size)
                .unwrap_or(0) as usize;
            let element_count = element_count(&array_sizes);

            if size != size_from_reflection * element_count {
                return Err(format!(
                    "Found a mismatch between logic and compiled spv alignments"
                ));
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
    use crate::codegen::{create_builtin_type_lookup, create_user_type_lookup};
    use crate::parse_source::FileToProcess;

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
        let mut compiler = shaderc::Compiler::new().unwrap();
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
