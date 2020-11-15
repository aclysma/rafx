
use serde::Deserialize;

use super::Declaration;
use super::Annotation;
use fnv::{FnvHashSet, FnvHashMap};
use std::collections::VecDeque;
use std::sync::Arc;


#[derive(Default, Deserialize, Debug)]
#[serde(rename = "export")]
struct ExportAnnotation(String);

#[derive(Default, Deserialize, Debug)]
#[serde(rename = "export")]
struct UseInternalBufferAnnotation(usize);


fn parse_ron_or_default<'de, T: Default + Deserialize<'de>>(data: &'de str) -> Result<T, String> {
    if !data.is_empty() {
        ron::de::from_str(&data)
            .map_err(|e| format!("Failed to parse annotation data. It should be an instance of '{}' encoded as RON.\n    Error: '{}'\n    Annotation Data: '{}'", core::any::type_name::<T>(), e, data))
    } else {
        Ok(Default::default())
    }
}

#[derive(Default, Debug)]
struct StructAnnotations {
    export: Option<ExportAnnotation>,
}

impl StructAnnotations {
    fn new(annotations: &[Annotation]) -> Result<Self, String> {
        let mut parsed_annotations = StructAnnotations::default();

        for annotation in annotations {
            let mut position = 0;
            let annotation_name = crate::parse::try_consume_identifier(&annotation.text, &mut position).ok_or("Failed to read annotation name")?;

            //let annotation_name = crate::parse::characters_to_string(&annotation.text[name_begin..name_end]);
            let annotation_data = crate::parse::characters_to_string(&annotation.text[position..]);

            //println!("name: {} data: {}", annotation_name, annotation_data);

            match annotation_name.as_str() {
                "export" => {
                    parsed_annotations.export = Some(parse_ron_or_default(&annotation_data)?);
                },
                _ => {
                    return Err(format!("Annotation named '{}' not allowed for structs", annotation_name));
                }
            }
        }

        Ok(parsed_annotations)
    }
}

#[derive(Default, Debug)]
struct BindingAnnotations {
    export: Option<ExportAnnotation>,
    use_internal_buffer: Option<UseInternalBufferAnnotation>
}

impl BindingAnnotations {
    fn new(annotations: &[Annotation]) -> Result<Self, String> {
        let mut parsed_annotations = BindingAnnotations::default();

        for annotation in annotations {
            let mut position = 0;
            let annotation_name = crate::parse::try_consume_identifier(&annotation.text, &mut position).ok_or("Failed to read annotation name")?;

            //let annotation_name = crate::parse::characters_to_string(&annotation.text[name_begin..name_end]);
            let annotation_data = crate::parse::characters_to_string(&annotation.text[position..]);

            //println!("name: {} data: {}", annotation_name, annotation_data);

            match annotation_name.as_str() {
                "export" => {
                    parsed_annotations.export = Some(parse_ron_or_default(&annotation_data)?);
                },
                "use_internal_buffer" => {
                    parsed_annotations.use_internal_buffer = Some(parse_ron_or_default(&annotation_data)?);
                },
                _ => {
                    return Err(format!("Annotation named '{}' not allowed for bindings", annotation_name));
                }
            }
        }

        Ok(parsed_annotations)
    }
}

#[derive(Debug, Clone)]
struct ParseFieldResult {
    type_name: String,
    field_name: String,
    array_sizes: Vec<usize>
}

#[derive(Debug)]
struct ParseStructResult {
    type_name: String,
    fields: Arc<Vec<ParseFieldResult>>,
    instance_name: Option<String>
}

fn parse_array_sizes(code: &[char], position: &mut usize) -> Result<Vec<usize>, String> {
    let mut array_sizes = Vec::<usize>::default();
    while crate::parse::try_consume_literal(code, position, "[").is_some() {
        crate::parse::skip_whitespace(code, position);
        let array_index = crate::parse::try_consume_array_index(code, position).ok_or(format!("Invalid array count while parsing struct field:\n{}", crate::parse::characters_to_string(&code)))?;
        array_sizes.push(array_index);
        crate::parse::skip_whitespace(code, position);
        crate::parse::try_consume_literal(code, position, "]").ok_or(format!("Missing ] on array count while parsing struct field:\n{}", crate::parse::characters_to_string(&code)))?;
        crate::parse::skip_whitespace(code, position);
    }

    Ok(array_sizes)
}

fn parse_field(code: &[char], position: &mut usize) -> Result<ParseFieldResult, String> {
    // Consume the field's type
    let field_type_name = crate::parse::try_consume_identifier(code, position).ok_or(format!("Failed to read field's type:\n{}", crate::parse::characters_to_string(&code)))?;
    crate::parse::skip_whitespace(code, position);

    // Consume the field's name
    let field_name = crate::parse::try_consume_identifier(code, position).ok_or(format!("Failed to read field's name:\n{}", crate::parse::characters_to_string(&code)))?;
    crate::parse::skip_whitespace(code, position);

    if *position >= code.len() {
        return Err(format!("Missing ; while parsing struct field:\n{}", crate::parse::characters_to_string(&code)));
    }

    let mut array_sizes = parse_array_sizes(code, position)?;

    crate::parse::try_consume_literal(code, position, ";").ok_or(format!("Missing ; while parsing struct field:\n{}", crate::parse::characters_to_string(&code)))?;

    Ok(ParseFieldResult {
        type_name: field_type_name,
        field_name,
        array_sizes
    })
}

fn try_parse_fields(code: &[char], position: &mut usize) -> Result<Option<Arc<Vec<ParseFieldResult>>>, String> {
    // Consume the opening {
    if crate::parse::try_consume_literal(code, position, "{").is_none() {
        return Ok(None);
    }

    // if *position >= code.len() || code[*position] != '{' {
    //     return Err(format!("Expected {{ while parsing struct:\n{}", crate::parse::characters_to_string(&code)));
    // }
    // *position += 1;

    let mut fields = Vec::default();

    // Consume struct fields
    while *position < code.len() {
        // We either just consumed the opening { or finished reading a field from the struct. Step
        // forward to either another field or the closing }
        crate::parse::skip_whitespace(code, position);
        if *position >= code.len() {
            return Err(format!("Missing closing }} while parsing struct:\n{}", crate::parse::characters_to_string(&code)));
        }

        // Stop if we encounter the closing }
        if crate::parse::try_consume_literal(code, position, "}").is_some() {
            break;
        }

        let field = parse_field(code, position)?;
        fields.push(field);
    }

    Ok(Some(Arc::new(fields)))
}

fn try_parse_struct(code: &[char]) -> Result<Option<ParseStructResult>, String> {
    let mut position = 0;

    // Consume the struct keyword. If it's missing, assume this isn't a struct and return None
    let consumed = crate::parse::try_consume_identifier(code, &mut position);
    if consumed.is_none() || consumed.unwrap() != "struct" {
        return Ok(None);
    }

    // Consume the name of the struct and all whitespace to the opening {
    crate::parse::skip_whitespace(code, &mut position);
    let type_name = crate::parse::try_consume_identifier(code, &mut position).ok_or(format!("Expected name of struct while parsing struct:\n{}", crate::parse::characters_to_string(&code)))?;

    crate::parse::skip_whitespace(code, &mut position);
    let fields = try_parse_fields(code, &mut position)?.ok_or(format!("Expected {{ while parsing struct:\n{}", crate::parse::characters_to_string(&code)))?;

    // an optional instance name
    crate::parse::skip_whitespace(code, &mut position);
    let instance_name = crate::parse::try_consume_identifier(code, &mut position);

    crate::parse::skip_whitespace(code, &mut position);
    crate::parse::try_consume_literal(code, &mut position, ";").ok_or(format!("Expected ; at end of struct:\n{}", crate::parse::characters_to_string(&code)))?;

    Ok(Some(ParseStructResult {
        type_name,
        fields,
        instance_name
    }))
}

#[derive(Debug)]
struct LayoutPart {
    key: String,
    value: Option<String>
}

#[derive(Debug, PartialEq)]
enum BindingType {
    Uniform,
    Buffer,
    In,
    Out
}

#[derive(Debug)]
struct ParseBindingResult {
    layout_parts: Vec<LayoutPart>,
    binding_type: BindingType,
    type_name: String,
    fields: Option<Arc<Vec<ParseFieldResult>>>,
    instance_name: String,
    array_sizes: Vec<usize>
}

fn parse_layout_part(code: &[char], position: &mut usize) -> Result<LayoutPart, String> {
    crate::parse::skip_whitespace(code, position);
    let key = crate::parse::try_consume_identifier(code, position).ok_or(format!("Expected key while parsing layout clause:\n{}", crate::parse::characters_to_string(&code)))?;
    crate::parse::skip_whitespace(code, position);
    if crate::parse::try_consume_literal(code, position, "=").is_some() {
        crate::parse::skip_whitespace(code, position);
        let value = crate::parse::try_consume_identifier(code, position).ok_or(format!("Expected value after = while parsing layout clause:\n{}", crate::parse::characters_to_string(&code)))?;

        Ok(LayoutPart {
            key,
            value: Some(value)
        })
    } else {
        Ok(LayoutPart {
            key,
            value: None
        })
    }
}

fn parse_layout_parts(code: &[char], position: &mut usize) -> Result<Vec<LayoutPart>, String> {
    let mut layout_parts = Vec::default();
    loop {
        if *position >= code.len() {
            return Err(format!("Expected closing ) while parsing binding:\n{}", crate::parse::characters_to_string(&code)));
        }

        // Covers immediate open and close i.e. layout () ...
        if crate::parse::try_consume_literal(code, position, ")").is_some() {
            break;
        }

        layout_parts.push(parse_layout_part(code, position)?);
        crate::parse::skip_whitespace(code, position);

        // Bail if we're at the end
        if crate::parse::try_consume_literal(code, position, ")").is_some() {
            break;
        }

        // Otherwise, consume a comma
        crate::parse::try_consume_literal(code, position, ",").ok_or(format!("Expected , between key/value pairs while parsing binding:\n{}", crate::parse::characters_to_string(&code)))?;
        crate::parse::skip_whitespace(code, position);
    }

    Ok(layout_parts)
}

fn try_parse_binding(code: &[char]) -> Result<Option<ParseBindingResult>, String> {
    let mut position = 0;

    // Consume the layout keyword. If it's missing, assume this isn't a binding and return None
    if crate::parse::try_consume_literal(code, &mut position, "layout").is_none() {
        return Ok(None);
    }

    crate::parse::skip_whitespace(code, &mut position);
    crate::parse::try_consume_literal(code, &mut position, "(").ok_or(format!("Expected opening ( while parsing binding:\n{}", crate::parse::characters_to_string(&code)))?;
    crate::parse::skip_whitespace(code, &mut position);

    let layout_parts = parse_layout_parts(code, &mut position)?;
    crate::parse::skip_whitespace(code, &mut position);

    // Either get the uniform or buffer keyword
    let binding_type = crate::parse::try_consume_identifier(code, &mut position).ok_or(format!("Expected keyword such as uniform, buffer, or in after layout in binding:\n{}", crate::parse::characters_to_string(&code)))?;
    let binding_type = match binding_type.as_str() {
        "uniform" => BindingType::Uniform,
        "buffer" => BindingType::Buffer,
        "in" => BindingType::In,
        "out" => BindingType::Out,
        _ => {
            return Err(format!("Expected keyword such as uniform, buffer, or in after layout in binding:\n{}", crate::parse::characters_to_string(&code)));
        }
    };

    crate::parse::skip_whitespace(code, &mut position);
    let type_name = crate::parse::try_consume_identifier(code, &mut position).ok_or(format!("Expected type name while parsing binding:\n{}", crate::parse::characters_to_string(&code)))?;

    crate::parse::skip_whitespace(code, &mut position);
    let fields = try_parse_fields(code, &mut position)?;

    crate::parse::skip_whitespace(code, &mut position);
    let instance_name = crate::parse::try_consume_identifier(code, &mut position).ok_or(format!("Expected instance name while parsing binding (required for exported bindings):\n{}", crate::parse::characters_to_string(&code)))?;

    let mut array_sizes = parse_array_sizes(code, &mut position)?;

    crate::parse::skip_whitespace(code, &mut position);
    crate::parse::try_consume_literal(code, &mut position, ";").ok_or(format!("Expected ; while parsing binding:\n{}", crate::parse::characters_to_string(&code)))?;

    // uniforms are std140 UNLESS they are push constants
    // buffers are std430, and push constants uniforms are std430
    // name types Uniform/Buffer/PushConstant?

    Ok(Some(ParseBindingResult {
        layout_parts,
        binding_type,
        type_name,
        fields,
        instance_name,
        array_sizes
    }))
}

fn try_parse_const(code: &[char]) -> Result<Option<()>, String> {
    let mut position = 0;

    // Consume the layout keyword. If it's missing, assume this isn't a binding and return None
    if crate::parse::try_consume_literal(code, &mut position, "const").is_none() {
        return Ok(None);
    }

    Ok(Some(()))
}

// fn generate_struct(result: &ParseStructResult, annotations: &StructAnnotations) -> Result<String, String> {
//     if !annotations.export.is_some() {
//         return Ok("".to_string())
//     }
//
// }

#[derive(Debug)]
enum StructOrBinding {
    Struct(usize),
    Binding(usize)
}

#[derive(Debug)]
struct TypeAlignmentInfo {
    rust_type: String,
    size: usize,
    std140_alignment: usize, // for structs/array elements, round up to multiple of 16
    std430_alignment: usize,
}

#[derive(Debug)]
struct UserType {
    struct_or_binding: StructOrBinding,
    type_name: String,
    fields: Arc<Vec<ParseFieldResult>>,
    export_name: Option<String>,
    export_uniform_layout: bool,
    export_push_constant_layout: bool,
    export_buffer_layout: bool,
}

fn recursive_modify_user_type<F: Fn(&mut UserType) -> bool>(user_types: &mut FnvHashMap::<String, UserType>, type_name: &str, f: &F) {
    let mut user_type = user_types.get_mut(type_name);
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


pub fn generate_rust_code(declarations: &[Declaration]) -> Result<String, String> {
    let mut structs = Vec::default();
    let mut bindings = Vec::default();

    //
    // Parse all declarations and their annotations
    //
    for declaration in declarations {
        if let Some(struct_result) = try_parse_struct(&declaration.text)? {
            //
            // Handle struct
            //
            //println!("Parsed a struct {:?}", struct_result);

            let struct_annotations = StructAnnotations::new(&declaration.annotations).map_err(|e| {
                format!(
                    "Failed to parse annotations for struct:\n\n{}\n\n{}",
                    crate::parse::characters_to_string(&declaration.text),
                    e,
                )
            })?;

            structs.push((struct_result, struct_annotations));

        } else if let Some(binding_result) = try_parse_binding(&declaration.text)? {
            //
            // Handle Binding
            //
            //println!("Parsed a binding {:?}", binding_result);

            let binding_annotations = BindingAnnotations::new(&declaration.annotations).map_err(|e| {
                format!(
                    "Failed to parse annotations for binding:\n\n{}\n\n{}",
                    crate::parse::characters_to_string(&declaration.text),
                    e,
                )
            })?;

            bindings.push((binding_result, binding_annotations));
        } else if try_parse_const(&declaration.text)?.is_some() {
            //
            // Stub for constants, not yet supported
            //
            if !declaration.annotations.is_empty() {
                return Err(format!("Annotations on consts not yet supported:\n{}", crate::parse::characters_to_string(&declaration.text)));
            }
        } else {
            return Err(format!("Annotations applied to declaration, but the declaration could not be parsed:\n{}", crate::parse::characters_to_string(&declaration.text)));
        }
    }

    //
    // Populate the user types map. Adding types in the map helps us detect duplicate type names
    // and quickly mark what layouts need to be exported (std140 - uniforms vs. std430 - push
    // constants/buffers)
    //
    // Structs and bindings can both declare new types, so gather data from both sources
    //
    let mut user_types = FnvHashMap::<String, UserType>::default();

    //
    // Populate user types from structs
    //
    for (index, (s, a)) in structs.iter().enumerate() {
        let export_name = a.export.as_ref().map(|x| x.0.clone());
        let old = user_types.insert(s.type_name.clone(), UserType {
            struct_or_binding: StructOrBinding::Struct(index),
            type_name: s.type_name.clone(),
            fields: s.fields.clone(),
            export_name,
            export_uniform_layout: false,
            export_push_constant_layout: false,
            export_buffer_layout: false,
        });

        if let Some(old) = old {
            return Err(format!("Duplicate user-defined type {}", s.type_name));
        }
    }

    //
    // Populate user types from bindings
    //
    for (index, (b, a)) in bindings.iter().enumerate() {
        if let Some(fields) = &b.fields {
            let export_name = a.export.as_ref().map(|x| x.0.clone());
            let old = user_types.insert(b.type_name.clone(), UserType {
                struct_or_binding: StructOrBinding::Binding(index),
                type_name: b.type_name.clone(),
                fields: fields.clone(),
                export_name,
                export_uniform_layout: false,
                export_push_constant_layout: false,
                export_buffer_layout: false,
            });

            if let Some(old) = old {
                return Err(format!("Duplicate user-defined type {}", b.type_name));
            }
        }
    }

    //
    // Any struct that's explicitly exported will produce all layouts
    //
    for (index, (s, a)) in structs.iter().enumerate() {
        if a.export.is_some() {
            recursive_modify_user_type(&mut user_types, &s.type_name, &|udt| {
                let already_marked = udt.export_uniform_layout && udt.export_push_constant_layout && udt.export_buffer_layout;
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
    for (index, (b, a)) in bindings.iter().enumerate() {
        if a.export.is_some() {
            if b.layout_parts.iter().any(|x| x.key == "push_constant") {
                recursive_modify_user_type(&mut user_types, &b.type_name, &|udt| {
                    let already_marked = udt.export_push_constant_layout;
                    udt.export_push_constant_layout = true;
                    !already_marked
                });
            } else if b.binding_type == BindingType::Uniform {
                recursive_modify_user_type(&mut user_types, &b.type_name, &|udt| {
                    let already_marked = udt.export_uniform_layout;
                    udt.export_uniform_layout = true;
                    !already_marked
                });
            } else if b.binding_type == BindingType::Buffer {
                recursive_modify_user_type(&mut user_types, &b.type_name, &|udt| {
                    let already_marked = udt.export_buffer_layout;
                    udt.export_buffer_layout = true;
                    !already_marked
                });
            }
        }
    }

    fn add_type_alignment_info(
        type_alignment_info: &mut FnvHashMap<String, TypeAlignmentInfo>,
        type_name: &str,
        rust_type: &str,
        size: usize,
        std140_alignment: usize,
        std430_alignment: usize,
    ) {
        let old = type_alignment_info.insert(type_name.to_string(), TypeAlignmentInfo {
            rust_type: rust_type.to_string(),
            size,
            std140_alignment,
            std430_alignment
        });
        assert!(old.is_none());
    }

    let mut builtin_types = FnvHashMap::<String, TypeAlignmentInfo>::default();
    add_type_alignment_info(&mut builtin_types, "uint", "u32", std::mem::size_of::<u32>(), 4, 4);
    add_type_alignment_info(&mut builtin_types, "bool", "bool", std::mem::size_of::<bool>(), 4, 4);
    add_type_alignment_info(&mut builtin_types, "float", "f32", std::mem::size_of::<f32>(), 4, 4);
    add_type_alignment_info(&mut builtin_types, "vec2", "[f32;2]", std::mem::size_of::<[f32;2]>(), 8, 8);
    add_type_alignment_info(&mut builtin_types, "vec3", "[f32;3]", std::mem::size_of::<[f32;3]>(), 16, 16);
    add_type_alignment_info(&mut builtin_types, "vec4", "[f32;4]", std::mem::size_of::<[f32;4]>(), 16, 16);

    for (type_name, user_type) in &user_types {
        println!("std140 Type info for {}", type_name);
        let alignment = determine_gpu_alignment(&builtin_types, &user_types, type_name, &[], StdAlignment::Std140)?;
        let size = determine_gpu_size(&builtin_types, &user_types, type_name, &[], 0, 0,&type_name, StdAlignment::Std140)?;
        println!("  {}: {} {}", type_name, alignment, size);

        println!("std430 Type info for {}", type_name);
        let alignment = determine_gpu_alignment(&builtin_types, &user_types, type_name, &[], StdAlignment::Std430)?;
        let size = determine_gpu_size(&builtin_types, &user_types, type_name, &[], 0, 0, &type_name, StdAlignment::Std430)?;
        println!("  {}: {} {}", type_name, alignment, size);



    }






    let mut rust_code = String::default();

    // for (type_name, user_type) in &user_types {
    //     if user_type.export_buffer_layout || user_type.export_push_constant_layout {
    //         generate_std430_layout(&user_types, type_name);
    //     }
    //
    //     if user_type.export_uniform_layout {
    //         generate_std140_layout(&user_types, type_name);
    //     }
    // }


    // let mut types_to_export_set : FnvHashSet::<String>::default();
    // let mut types_to_export_dependency_queue = VecDeque::<String>::default();
    // for (s, a) in &structs {
    //     if a.export.is_some() {
    //         types_to_export_set.insert(s.type_name.clone());
    //     }
    // }




    //for






    Ok(rust_code)
}

struct FieldVisitParams<'a> {
    gpu_type_name: &'a str,
    offset: usize
}

fn determine_gpu_size(
    builtin_types: &FnvHashMap<String, TypeAlignmentInfo>,
    user_types: &FnvHashMap<String, UserType>,
    query_type: &str,
    array_sizes: &[usize],
    mut offset: usize,
    logging_offset: usize,
    logging_name: &str,
    std_alignment: StdAlignment
) -> Result<usize, String> {
    // We only need to know how many elements we have
    let mut element_count = 1;
    for x in array_sizes {
        element_count *= x;
    }

    // Align this type (may be a struct, built-in, etc.
    let alignment = determine_gpu_alignment(builtin_types, user_types, query_type, array_sizes, std_alignment)?;
    offset = (offset + alignment - 1) / alignment * alignment;

    if let Some(builtin_type) = builtin_types.get(query_type) {
        offset = (offset + alignment - 1) / alignment * alignment;

        println!("  {} +{} (size: {}) [{} elements of size {}, alignment: {}, name: {}]", query_type, logging_offset, element_count * builtin_type.size, element_count, builtin_type.size, alignment, logging_name);
        if array_sizes.is_empty() {
            offset += builtin_type.size;
        } else {
            let padded_size = (builtin_type.size + alignment - 1) / alignment * alignment;
            offset += padded_size * element_count;
        }

        Ok(offset)
    } else if let Some(user_type) = user_types.get(query_type) {
        let mut offset_within_struct = 0;
        println!("  process fields for {}", logging_name);
        for f in &*user_type.fields {
            // Align the member
            let field_alignment = determine_gpu_alignment(builtin_types, user_types, &f.type_name, &f.array_sizes, std_alignment)?;
            offset_within_struct = (offset_within_struct + field_alignment - 1) / field_alignment * field_alignment;

            offset_within_struct = determine_gpu_size(builtin_types, user_types, &f.type_name, &f.array_sizes, offset_within_struct, offset + offset_within_struct, &f.field_name, std_alignment)?;
        }

        let padded_size = (offset_within_struct + alignment - 1) / alignment * alignment;
        println!("    struct {} total size: {} [{} elements of size {}]", logging_name, padded_size * element_count, element_count, padded_size);
        offset += padded_size * element_count;

        // // the base offset of the member following the sub-structure is rounded up to the next multiple of the base alignment of the structure
        // offset = (offset + alignment - 1) / alignment * alignment;
        Ok(offset)
    } else {
        return Err(format!("Could not find type {}", query_type));
    }
}

#[derive(Copy, Clone)]
enum StdAlignment {
    Std140,
    Std430
}

fn determine_gpu_alignment(
    builtin_types: &FnvHashMap<String, TypeAlignmentInfo>,
    user_types: &FnvHashMap<String, UserType>,
    query_type: &str,
    array_sizes: &[usize],
    alignment: StdAlignment
) -> Result<usize, String> {
    match alignment {
        StdAlignment::Std140 => determine_gpu_alignment_std140(builtin_types, user_types, query_type, array_sizes),
        StdAlignment::Std430 => determine_gpu_alignment_std430(builtin_types, user_types, query_type, array_sizes),
    }
}

//TODO: Do I need to generate structs for array elements that are not properly aligned?
fn determine_gpu_alignment_std140(
    builtin_types: &FnvHashMap<String, TypeAlignmentInfo>,
    user_types: &FnvHashMap<String, UserType>,
    query_type: &str,
    array_sizes: &[usize]
) -> Result<usize, String> {
    if let Some(builtin_type) = builtin_types.get(query_type) {
        if !array_sizes.is_empty() {
            // For std140, array element alignment is rounded up element to multiple of 16
            Ok((builtin_type.std140_alignment + 15) / 16 * 16)
        } else {
            // Built-ins that are not array elements get normal alignment
            Ok(builtin_type.std140_alignment)
        }
    } else if let Some(user_type) = user_types.get(query_type) {
        let mut alignment = 16;
        for f in &*user_type.fields {
            let field_alignment = determine_gpu_alignment_std140(builtin_types, user_types, &f.type_name, &f.array_sizes)?;

            // For std140, struct alignment is the max of its field's alignment requirements, rounded
            // up to 16
            //let field_alignment = (field_alignment + 15) / 16 * 16;
            alignment = alignment.max(field_alignment);
        }

        Ok((alignment + 15) / 16 * 16)
    } else {
        return Err(format!("Could not find type {}", query_type));
    }
}

fn determine_gpu_alignment_std430(
    builtin_types: &FnvHashMap<String, TypeAlignmentInfo>,
    user_types: &FnvHashMap<String, UserType>,
    query_type: &str,
    _array_sizes: &[usize]
) -> Result<usize, String> {
    if let Some(builtin_type) = builtin_types.get(query_type) {
        Ok(builtin_type.std430_alignment)
    } else if let Some(user_type) = user_types.get(query_type) {
        let mut alignment = 4;
        for f in &*user_type.fields {
            let field_alignment = determine_gpu_alignment_std430(builtin_types, user_types, &f.type_name, &f.array_sizes)?;
            alignment = alignment.max(field_alignment);
        }

        Ok(alignment)
    } else {
        return Err(format!("Could not find type {}", query_type));
    }
}
