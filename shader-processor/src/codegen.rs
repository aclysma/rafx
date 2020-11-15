
use serde::Deserialize;

use super::Declaration;
use super::Annotation;


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

#[derive(Debug)]
struct ParseFieldResult {
    type_name: String,
    field_name: String,
    array_sizes: Vec<usize>
}

#[derive(Debug)]
struct ParseStructResult {
    type_name: String,
    fields: Vec<ParseFieldResult>,
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

fn try_parse_fields(code: &[char], position: &mut usize) -> Result<Option<Vec<ParseFieldResult>>, String> {
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

    Ok(Some(fields))
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

#[derive(Debug)]
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
    fields: Option<Vec<ParseFieldResult>>,
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

pub fn generate_rust_code(declarations: &[Declaration]) -> Result<String, String> {
    let mut structs = Vec::default();
    let mut bindings = Vec::default();

    for declaration in declarations {
        if let Some(struct_result) = try_parse_struct(&declaration.text)? {
            println!("Parsed a struct {:?}", struct_result);

            let struct_annotations = StructAnnotations::new(&declaration.annotations).map_err(|e| {
                format!(
                    "Failed to parse annotations for struct:\n\n{}\n\n{}",
                    crate::parse::characters_to_string(&declaration.text),
                    e,
                )
            })?;

            //rust_code += &generate_struct(&struct_result, &struct_annotations)?;
            structs.push((struct_result, struct_annotations));

        } else if let Some(binding_result) = try_parse_binding(&declaration.text)? {
            println!("Parsed a binding {:?}", binding_result);


            let binding_annotations = BindingAnnotations::new(&declaration.annotations).map_err(|e| {
                format!(
                    "Failed to parse annotations for binding:\n\n{}\n\n{}",
                    crate::parse::characters_to_string(&declaration.text),
                    e,
                )
            })?;

            bindings.push((binding_result, binding_annotations));
        } else if try_parse_const(&declaration.text)?.is_some() {
            if !declaration.annotations.is_empty() {
                return Err(format!("Annotations on consts not yet supported:\n{}", crate::parse::characters_to_string(&declaration.text)));
            }
        } else {
            return Err(format!("Annotations applied to declaration, but the declaration could not be parsed:\n{}", crate::parse::characters_to_string(&declaration.text)));
        }
    }

    let mut rust_code = String::default();


    //for






    Ok(rust_code)
}