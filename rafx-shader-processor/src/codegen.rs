use crate::parse_declarations::{
    BindingType, ParseDeclarationsResult, ParsedBindingWithAnnotations,
};
use crate::shader_types::*;
use fnv::{FnvHashMap, FnvHashSet};
use rafx_api::RafxResourceType;
use rafx_framework::reflected_shader::ReflectedEntryPoint;
use std::collections::BTreeMap;

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

pub(crate) fn generate_rust_code(
    builtin_types: &FnvHashMap<String, TypeAlignmentInfo>,
    user_types: &mut FnvHashMap<String, UserType>,
    parsed_declarations: &ParseDeclarationsResult,
    shader_module: &spirv_reflect::ShaderModule,
    reflected_entry_point: &ReflectedEntryPoint,
    for_rafx_framework_crate: bool,
) -> Result<String, String> {
    //
    // Populate the user types map. Adding types in the map helps us detect duplicate type names
    // and quickly mark what layouts need to be exported (std140 - uniforms vs. std430 - push
    // constants/buffers)
    //
    // Structs and bindings can both declare new types, so gather data from both sources
    //

    verify_all_binding_layouts(&builtin_types, user_types, shader_module)?;

    //
    // Any struct that's explicitly exported will produce all layouts
    //
    for s in &parsed_declarations.structs {
        if s.annotations.export.is_some() {
            recursive_modify_user_type(user_types, &s.parsed.type_name, &|udt| {
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
                    recursive_modify_user_type(user_types, &b.parsed.type_name, &|udt| {
                        let already_marked = udt.export_push_constant_layout;
                        udt.export_push_constant_layout = true;
                        !already_marked
                    });
                }
                StructBindingType::Uniform => {
                    recursive_modify_user_type(user_types, &b.parsed.type_name, &|udt| {
                        let already_marked = udt.export_uniform_layout;
                        udt.export_uniform_layout = true;
                        !already_marked
                    });
                }
                StructBindingType::Buffer => {
                    recursive_modify_user_type(user_types, &b.parsed.type_name, &|udt| {
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
        for_rafx_framework_crate,
    )
}

fn generate_rust_file(
    parsed_declarations: &ParseDeclarationsResult,
    builtin_types: &FnvHashMap<String, TypeAlignmentInfo>,
    user_types: &FnvHashMap<String, UserType>,
    reflected_entry_point: &ReflectedEntryPoint,
    for_rafx_framework_crate: bool,
) -> Result<String, String> {
    let mut rust_code = Vec::<String>::default();

    rust_header(&mut rust_code, for_rafx_framework_crate);

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

fn rust_header(
    rust_code: &mut Vec<String>,
    for_rafx_framework_crate: bool,
) {
    rust_code.push("// This code is auto-generated by the shader processor.\n\n".to_string());

    if for_rafx_framework_crate {
        rust_code.push("#[allow(unused_imports)]\n".to_string());
        rust_code.push("use rafx_api::RafxResult;\n\n".to_string());
        rust_code.push("#[allow(unused_imports)]\n".to_string());
        rust_code.push("use crate::{ResourceArc, ImageViewResource, DynDescriptorSet, DescriptorSetAllocator, DescriptorSetInitializer, DescriptorSetArc, DescriptorSetWriter, DescriptorSetWriterContext, DescriptorSetBindings};\n\n".to_string());
    } else {
        rust_code.push("#[allow(unused_imports)]\n".to_string());
        rust_code.push("use rafx::RafxResult;\n\n".to_string());
        rust_code.push("#[allow(unused_imports)]\n".to_string());
        rust_code.push("use rafx::framework::{ResourceArc, ImageViewResource, DynDescriptorSet, DescriptorSetAllocator, DescriptorSetInitializer, DescriptorSetArc, DescriptorSetWriter, DescriptorSetWriterContext, DescriptorSetBindings};\n\n".to_string());
    }
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
            rust_code.push(generate_struct_code(&s));
            rust_code.push(generate_struct_default_code(&s));
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
            rust_code.push(generate_struct_code(&s));
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
        rust_code.push("    fn create_descriptor_set(descriptor_set_allocator: &mut DescriptorSetAllocator, descriptor_set: DynDescriptorSet, args: Self) -> RafxResult<DescriptorSetArc> {\n".to_string());
        rust_code.push(
            "        let mut descriptor = Self::create_dyn_descriptor_set(descriptor_set, args);\n"
                .to_string(),
        );
        rust_code.push("        descriptor.0.flush(descriptor_set_allocator)?;\n".to_string());
        rust_code.push("        Ok(descriptor.0.descriptor_set().clone())\n".to_string());
        rust_code.push("    }\n".to_string());

        rust_code.push("}\n\n".to_string());

        //
        // DescriptorSetWriter trait impl
        //

        rust_code.push(format!(
            "impl<'a> DescriptorSetWriter<'a> for {}{} {{\n",
            args_struct_name, wrapper_args_generic_params
        ));

        // write_to
        rust_code.push(
            "    fn write_to(descriptor_set: &mut DescriptorSetWriterContext, args: Self) {\n"
                .to_string(),
        );
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
        rust_code.push("    pub fn flush(&mut self, descriptor_set_allocator: &mut DescriptorSetAllocator) -> RafxResult<()> {\n".to_string());
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
        .find(|x| x.resource.binding == binding_index as u32)
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

    if e.immutable_samplers.is_none()
        && e.resource.resource_type == RafxResourceType::COMBINED_IMAGE_SAMPLER
    {
        Err("Combined image samplers only supported with immutable samplers")?;
    }

    match e.resource.resource_type {
        RafxResourceType::SAMPLER => {
            if e.immutable_samplers.is_none() {
                // TODO: Generate a setter for samplers
            }
        }
        RafxResourceType::TEXTURE
        | RafxResourceType::TEXTURE_READ_WRITE
        | RafxResourceType::COMBINED_IMAGE_SAMPLER => {
            if e.resource.element_count_normalized() > 1 {
                binding_wrapper_items.push(BindingWrapperItem {
                    binding_name,
                    setter_fn_name_single: "set_image_at_index".to_string(),
                    setter_fn_name_multi: "set_images".to_string(),
                    args_struct_member_type: format!(
                        "&'a [Option<&'a ResourceArc<ImageViewResource>>; {}]",
                        e.resource.element_count_normalized()
                    )
                    .to_string(),
                    set_element_param_type_single: "&ResourceArc<ImageViewResource>".to_string(),
                    set_element_param_type_multi: format!(
                        "&[Option<& ResourceArc<ImageViewResource>>; {}]",
                        e.resource.element_count_normalized()
                    )
                    .to_string(),
                    binding_index_string,
                    descriptor_count: e.resource.element_count_normalized(),
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
                    descriptor_count: e.resource.element_count_normalized(),
                });
            }
            binding_wrapper_struct_lifetimes.push("'a".to_string());
        }
        RafxResourceType::UNIFORM_BUFFER
        | RafxResourceType::BUFFER
        | RafxResourceType::BUFFER_READ_WRITE => {
            assert_eq!(e.resource.element_count_normalized(), 1);
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
                descriptor_count: e.resource.element_count_normalized(),
            });
            binding_wrapper_struct_lifetimes.push("'a".to_string());
        }
        // No support for these yet
        // RafxResourceType::UniformBufferDynamic => {}
        // RafxResourceType::StorageBufferDynamic => {}
        // RafxResourceType::UniformTexelBuffer => {}
        // RafxResourceType::StorageTexelBuffer => {}
        // RafxResourceType::InputAttachment => {}
        _ => {
            Err(format!(
                "Unsupported resource type {:?}",
                e.resource.resource_type
            ))?;
        }
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
            rust_code.push(generate_struct_test_code(&s));
        }
        rust_code.push("}\n".to_string());
    }
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

fn generate_struct_code(st: &GenerateStructResult) -> String {
    let mut result_string = String::default();
    result_string += &format!(
        "#[derive(Copy, Clone, Debug)]\n#[repr(C)]\npub struct {} {{\n",
        st.name
    );
    for m in &st.members {
        result_string += &format_member(&m.name, &m.ty, m.offset, m.size);
    }
    result_string += &format!("}} // {} bytes\n\n", st.size);
    result_string
}

fn generate_struct_default_code(st: &GenerateStructResult) -> String {
    let mut result_string = String::default();
    result_string += &format!("impl Default for {} {{\n", st.name);
    result_string += &format!("    fn default() -> Self {{\n");
    result_string += &format!("        {} {{\n", st.name);
    for m in &st.members {
        //result_string += &format!("            {}: {}::default(),\n", &m.name, &m.ty);
        result_string += &format!("            {}: {},\n", &m.name, m.default_value);
    }
    result_string += &format!("        }}\n");
    result_string += &format!("    }}\n");
    result_string += &format!("}}\n\n");
    result_string
}

fn generate_struct_test_code(st: &GenerateStructResult) -> String {
    use heck::SnakeCase;
    let mut result_string = String::default();
    result_string += &format!(
        "\n    #[test]\n    fn test_struct_{}() {{\n",
        st.name.to_snake_case()
    );
    result_string += &format!(
        "        assert_eq!(std::mem::size_of::<{}>(), {});\n",
        st.name, st.size
    );
    for m in &st.members {
        result_string += &format!(
            "        assert_eq!(std::mem::size_of::<{}>(), {});\n",
            m.ty, m.size
        );
        result_string += &format!(
            "        assert_eq!(std::mem::align_of::<{}>(), {});\n",
            m.ty, m.align
        );

        // Very large structs may be larger than can fit on the stack, which doesn't work with memoffset::offset_of!()
        if st.size < (1024 * 1024) {
            result_string += &format!(
                "        assert_eq!(memoffset::offset_of!({}, {}), {});\n",
                st.name, m.name, m.offset
            );
        }
    }
    result_string += &format!("    }}\n");
    result_string
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
