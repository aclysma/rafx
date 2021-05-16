use serde::Deserialize;

use super::AnnotationText;
use super::DeclarationText;
use std::num::ParseIntError;
use std::sync::Arc;

use rafx_api::RafxSamplerDef;

#[derive(Default, Deserialize, Debug)]
#[serde(rename = "export")]
pub(crate) struct ExportAnnotation(/*pub(crate) String*/);

#[derive(Default, Deserialize, Debug)]
#[serde(rename = "internal_buffer")]
pub(crate) struct UseInternalBufferAnnotation(/*pub(crate) u32*/);

#[derive(Default, Deserialize, Debug)]
#[serde(rename = "immutable_samplers")]
pub(crate) struct ImmutableSamplersAnnotation(pub(crate) Vec<RafxSamplerDef>);

#[derive(Default, Deserialize, Debug)]
#[serde(rename = "slot_name")]
pub(crate) struct SlotNameAnnotation(pub(crate) String);

#[derive(Default, Deserialize, Debug)]
#[serde(rename = "semantic")]
pub(crate) struct SemanticAnnotation(pub(crate) String);

fn parse_ron_or_default<'de, T: Default + Deserialize<'de>>(data: &'de str) -> Result<T, String> {
    if !data.is_empty() {
        ron::de::from_str(&data)
            .map_err(|e| format!("Failed to parse annotation data. It should be an instance of '{}' encoded as RON.\n    Error: '{}'\n    Annotation Data: '{}'", core::any::type_name::<T>(), e, data))
    } else {
        Ok(Default::default())
    }
}

#[derive(Default, Debug)]
pub(crate) struct StructAnnotations {
    pub(crate) export: Option<ExportAnnotation>,
}

impl StructAnnotations {
    fn new(annotations: &[AnnotationText]) -> Result<Self, String> {
        let mut parsed_annotations = StructAnnotations::default();

        for annotation in annotations {
            let mut position = 0;
            let annotation_name =
                crate::parse_source::try_consume_identifier(&annotation.text, &mut position)
                    .ok_or("Failed to read annotation name")?;

            //let annotation_name = crate::parse::characters_to_string(&annotation.text[name_begin..name_end]);
            let annotation_data =
                crate::parse_source::characters_to_string(&annotation.text[position..]);

            //println!("name: {} data: {}", annotation_name, annotation_data);

            match annotation_name.as_str() {
                "export" => {
                    parsed_annotations.export = Some(parse_ron_or_default(&annotation_data)?);
                }
                _ => {
                    return Err(format!(
                        "Annotation named '{}' not allowed for structs",
                        annotation_name
                    ));
                }
            }
        }

        Ok(parsed_annotations)
    }
}

#[derive(Default, Debug)]
pub(crate) struct BindingAnnotations {
    pub(crate) export: Option<ExportAnnotation>,
    pub(crate) use_internal_buffer: Option<UseInternalBufferAnnotation>,
    pub(crate) immutable_samplers: Option<ImmutableSamplersAnnotation>,
    pub(crate) slot_name: Option<SlotNameAnnotation>,
    pub(crate) semantic: Option<SemanticAnnotation>,
}

impl BindingAnnotations {
    fn new(annotations: &[AnnotationText]) -> Result<Self, String> {
        let mut parsed_annotations = BindingAnnotations::default();

        for annotation in annotations {
            let mut position = 0;
            let annotation_name =
                crate::parse_source::try_consume_identifier(&annotation.text, &mut position)
                    .ok_or("Failed to read annotation name")?;

            //let annotation_name = crate::parse::characters_to_string(&annotation.text[name_begin..name_end]);
            let annotation_data =
                crate::parse_source::characters_to_string(&annotation.text[position..]);

            //println!("name: {} data: {}", annotation_name, annotation_data);

            match annotation_name.as_str() {
                "export" => {
                    parsed_annotations.export = Some(parse_ron_or_default(&annotation_data)?);
                }
                "internal_buffer" => {
                    parsed_annotations.use_internal_buffer =
                        Some(parse_ron_or_default(&annotation_data)?);
                }
                "immutable_samplers" => {
                    parsed_annotations.immutable_samplers =
                        Some(parse_ron_or_default(&annotation_data)?);
                }
                "slot_name" => {
                    parsed_annotations.slot_name = Some(parse_ron_or_default(&annotation_data)?);
                }
                "semantic" => {
                    parsed_annotations.semantic = Some(parse_ron_or_default(&annotation_data)?);
                }
                _ => {
                    return Err(format!(
                        "Annotation named '{}' not allowed for bindings",
                        annotation_name
                    ));
                }
            }
        }

        Ok(parsed_annotations)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ParseFieldResult {
    pub(crate) type_name: String,
    pub(crate) field_name: String,
    pub(crate) array_sizes: Vec<usize>,
}

#[derive(Debug)]
pub(crate) struct ParseStructResult {
    pub(crate) type_name: String,
    pub(crate) fields: Arc<Vec<ParseFieldResult>>,
    pub(crate) instance_name: Option<String>,
}

fn parse_array_sizes(
    constants: &[ParseConstantResult],
    code: &[char],
    position: &mut usize,
) -> Result<Vec<usize>, String> {
    let mut array_sizes = Vec::<usize>::default();
    while crate::parse_source::try_consume_literal(code, position, "[").is_some() {
        crate::parse_source::skip_whitespace(code, position);
        let array_index =
            crate::parse_source::try_consume_array_index(constants, code, position).unwrap_or(0);
        array_sizes.push(array_index);
        crate::parse_source::skip_whitespace(code, position);
        crate::parse_source::try_consume_literal(code, position, "]").ok_or(format!(
            "Missing ] on array count while parsing struct field:\n{}",
            crate::parse_source::characters_to_string(&code)
        ))?;
        crate::parse_source::skip_whitespace(code, position);
    }

    Ok(array_sizes)
}

fn parse_field(
    constants: &[ParseConstantResult],
    code: &[char],
    position: &mut usize,
) -> Result<ParseFieldResult, String> {
    // Consume the field's type
    let field_type_name =
        crate::parse_source::try_consume_identifier(code, position).ok_or(format!(
            "Failed to read field's type:\n{}",
            crate::parse_source::characters_to_string(&code)
        ))?;
    crate::parse_source::skip_whitespace(code, position);

    // Consume the field's name
    let field_name = crate::parse_source::try_consume_identifier(code, position).ok_or(format!(
        "Failed to read field's name:\n{}",
        crate::parse_source::characters_to_string(&code)
    ))?;
    crate::parse_source::skip_whitespace(code, position);

    if *position >= code.len() {
        return Err(format!(
            "Missing ; while parsing struct field:\n{}",
            crate::parse_source::characters_to_string(&code)
        ));
    }

    let array_sizes = parse_array_sizes(constants, code, position)?;

    crate::parse_source::try_consume_literal(code, position, ";").ok_or(format!(
        "Missing ; while parsing struct field:\n{}",
        crate::parse_source::characters_to_string(&code)
    ))?;

    Ok(ParseFieldResult {
        type_name: field_type_name,
        field_name,
        array_sizes,
    })
}

fn try_parse_fields(
    constants: &[ParseConstantResult],
    code: &[char],
    position: &mut usize,
) -> Result<Option<Arc<Vec<ParseFieldResult>>>, String> {
    // Consume the opening {
    if crate::parse_source::try_consume_literal(code, position, "{").is_none() {
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
        crate::parse_source::skip_whitespace(code, position);
        if *position >= code.len() {
            return Err(format!(
                "Missing closing }} while parsing struct:\n{}",
                crate::parse_source::characters_to_string(&code)
            ));
        }

        // Stop if we encounter the closing }
        if crate::parse_source::try_consume_literal(code, position, "}").is_some() {
            break;
        }

        let field = parse_field(constants, code, position)?;
        fields.push(field);
    }

    Ok(Some(Arc::new(fields)))
}

fn try_parse_struct(
    constants: &[ParseConstantResult],
    code: &[char],
) -> Result<Option<ParseStructResult>, String> {
    let mut position = 0;

    // Consume the struct keyword. If it's missing, assume this isn't a struct and return None
    let consumed = crate::parse_source::try_consume_identifier(code, &mut position);
    if consumed.is_none() || consumed.unwrap() != "struct" {
        return Ok(None);
    }

    // Consume the name of the struct and all whitespace to the opening {
    crate::parse_source::skip_whitespace(code, &mut position);
    let type_name =
        crate::parse_source::try_consume_identifier(code, &mut position).ok_or(format!(
            "Expected name of struct while parsing struct:\n{}",
            crate::parse_source::characters_to_string(&code)
        ))?;

    crate::parse_source::skip_whitespace(code, &mut position);
    let fields = try_parse_fields(constants, code, &mut position)?.ok_or(format!(
        "Expected {{ while parsing struct:\n{}",
        crate::parse_source::characters_to_string(&code)
    ))?;

    // an optional instance name
    crate::parse_source::skip_whitespace(code, &mut position);
    let instance_name = crate::parse_source::try_consume_identifier(code, &mut position);

    crate::parse_source::skip_whitespace(code, &mut position);
    crate::parse_source::try_consume_literal(code, &mut position, ";").ok_or(format!(
        "Expected ; at end of struct:\n{}",
        crate::parse_source::characters_to_string(&code)
    ))?;

    Ok(Some(ParseStructResult {
        type_name,
        fields,
        instance_name,
    }))
}

#[derive(Debug)]
pub(crate) struct LayoutPart {
    pub(crate) key: String,
    pub(crate) value: Option<String>,
}

#[derive(Debug, PartialEq)]
pub(crate) enum BindingType {
    Uniform,
    Buffer,
    In,
    Out,
}

#[derive(Default, Debug)]
pub(crate) struct ParsedLayoutParts {
    pub(crate) set: Option<usize>,
    pub(crate) binding: Option<usize>,
    pub(crate) location: Option<usize>,
    pub(crate) push_constant: bool,
}

impl ParsedLayoutParts {
    fn from_parts(parts: &[LayoutPart]) -> Result<Self, String> {
        let mut parsed = ParsedLayoutParts::default();

        for part in parts {
            match part.key.as_str() {
                "push_constant" => parsed.push_constant = true,
                "set" => {
                    if parsed.set.is_some() {
                        return Err(
                            "layout parts for a binding defines set multiple times".to_string()
                        );
                    }

                    let set: usize = part
                        .value
                        .as_ref()
                        .ok_or_else(|| "set in layout but no index assigned".to_string())?
                        .parse()
                        .map_err(|x: ParseIntError| x.to_string())?;

                    parsed.set = Some(set)
                }
                "binding" => {
                    if parsed.binding.is_some() {
                        return Err(
                            "layout parts for a binding defines binding multiple times".to_string()
                        );
                    }

                    let binding: usize = part
                        .value
                        .as_ref()
                        .ok_or_else(|| "binding in layout but no index assigned".to_string())?
                        .parse()
                        .map_err(|x: ParseIntError| x.to_string())?;

                    parsed.binding = Some(binding)
                }
                "location" => {
                    if parsed.location.is_some() {
                        return Err("layout parts for a binding defines location multiple times"
                            .to_string());
                    }

                    let location: usize = part
                        .value
                        .as_ref()
                        .ok_or_else(|| "location in layout but no index assigned".to_string())?
                        .parse()
                        .map_err(|x: ParseIntError| x.to_string())?;

                    parsed.location = Some(location)
                }
                _ => {}
            }
        }

        Ok(parsed)
    }
}

#[derive(Debug)]
pub(crate) struct ParseBindingResult {
    pub(crate) layout_parts: ParsedLayoutParts,
    pub(crate) binding_type: BindingType,
    pub(crate) type_name: String,
    pub(crate) fields: Option<Arc<Vec<ParseFieldResult>>>,
    pub(crate) instance_name: String,
    pub(crate) array_sizes: Vec<usize>,
}

#[derive(Debug, Clone)]
pub(crate) struct ParseConstantResult {
    pub(crate) type_name: String,
    pub(crate) instance_name: String,
    pub(crate) value: Option<String>,
}

fn parse_layout_part(
    code: &[char],
    position: &mut usize,
) -> Result<LayoutPart, String> {
    crate::parse_source::skip_whitespace(code, position);
    let key = crate::parse_source::try_consume_identifier(code, position).ok_or(format!(
        "Expected key while parsing layout clause:\n{}",
        crate::parse_source::characters_to_string(&code)
    ))?;
    crate::parse_source::skip_whitespace(code, position);
    if crate::parse_source::try_consume_literal(code, position, "=").is_some() {
        crate::parse_source::skip_whitespace(code, position);
        let value = crate::parse_source::try_consume_identifier(code, position).ok_or(format!(
            "Expected value after = while parsing layout clause:\n{}",
            crate::parse_source::characters_to_string(&code)
        ))?;

        Ok(LayoutPart {
            key,
            value: Some(value),
        })
    } else {
        Ok(LayoutPart { key, value: None })
    }
}

fn parse_layout_parts(
    code: &[char],
    position: &mut usize,
) -> Result<Vec<LayoutPart>, String> {
    let mut layout_parts = Vec::default();
    loop {
        if *position >= code.len() {
            return Err(format!(
                "Expected closing ) while parsing binding:\n{}",
                crate::parse_source::characters_to_string(&code)
            ));
        }

        // Covers immediate open and close i.e. layout () ...
        if crate::parse_source::try_consume_literal(code, position, ")").is_some() {
            break;
        }

        layout_parts.push(parse_layout_part(code, position)?);
        crate::parse_source::skip_whitespace(code, position);

        // Bail if we're at the end
        if crate::parse_source::try_consume_literal(code, position, ")").is_some() {
            break;
        }

        // Otherwise, consume a comma
        crate::parse_source::try_consume_literal(code, position, ",").ok_or(format!(
            "Expected , between key/value pairs while parsing binding:\n{}",
            crate::parse_source::characters_to_string(&code)
        ))?;
        crate::parse_source::skip_whitespace(code, position);
    }

    Ok(layout_parts)
}

fn try_parse_binding(
    constants: &[ParseConstantResult],
    code: &[char],
) -> Result<Option<ParseBindingResult>, String> {
    let mut position = 0;

    //
    // See if it starts with layout. If not, assume this isn't a binding and return None
    //
    if crate::parse_source::try_consume_literal(code, &mut position, "layout").is_none() {
        return Ok(None);
    }

    //
    // Parse the (...) in the layout (...) prefix for this binding
    //
    crate::parse_source::skip_whitespace(code, &mut position);
    crate::parse_source::try_consume_literal(code, &mut position, "(").ok_or(format!(
        "Expected opening ( while parsing binding:\n{}",
        crate::parse_source::characters_to_string(&code)
    ))?;
    crate::parse_source::skip_whitespace(code, &mut position);

    let layout_parts = parse_layout_parts(code, &mut position)?;
    crate::parse_source::skip_whitespace(code, &mut position);

    //
    // Grab all the identifers, optionally grab struct fields, then try to grab one more identifier
    //
    let mut identifiers = Vec::default();
    while let Some(identifier) = crate::parse_source::try_consume_identifier(code, &mut position) {
        identifiers.push(identifier);
        crate::parse_source::skip_whitespace(code, &mut position);
    }

    // Optionally get struct fields
    let fields = try_parse_fields(constants, code, &mut position)?;
    if fields.is_some() {
        // If struct fields exist, we need one more identifier
        crate::parse_source::skip_whitespace(code, &mut position);
        let instance_name = crate::parse_source::try_consume_identifier(code, &mut position)
            .ok_or(format!(
            "Expected instance name while parsing binding (required for exported bindings):\n{}",
            crate::parse_source::characters_to_string(&code)
        ))?;
        identifiers.push(instance_name);
    }

    let modifiers = &identifiers[0..(identifiers.len() - 2)];
    let type_name = identifiers[identifiers.len() - 2].clone();
    let instance_name = identifiers[identifiers.len() - 1].clone();

    log::trace!(
        "parsing binding: type name: {}, instance_name: {}, modifiers: {:?}",
        type_name,
        instance_name,
        modifiers
    );

    let mut binding_type = None;
    for modifier in modifiers {
        let bt = match modifier.as_str() {
            "uniform" => Some(BindingType::Uniform),
            "buffer" => Some(BindingType::Buffer),
            "in" => Some(BindingType::In),
            "out" => Some(BindingType::Out),
            _ => None,
        };

        if bt.is_some() {
            if binding_type.is_some() {
                Err(format!(
                    "Multiple keywords indicating binding type (uniform/buffer/in/out) in binding:\n{}",
                    crate::parse_source::characters_to_string(&code)
                ))?
            }

            binding_type = bt;
        }
    }

    let binding_type = binding_type.ok_or_else(|| {
        format!(
            "Expected keyword indicating binding type (uniform/buffer/in/out) after layout in binding:\n{}",
            crate::parse_source::characters_to_string(&code)
        )
    })?;

    let array_sizes = parse_array_sizes(constants, code, &mut position)?;

    crate::parse_source::skip_whitespace(code, &mut position);
    crate::parse_source::try_consume_literal(code, &mut position, ";").ok_or(format!(
        "Expected ; while parsing binding:\n{}",
        crate::parse_source::characters_to_string(&code)
    ))?;

    // uniforms are std140 UNLESS they are push constants
    // buffers are std430, and push constants uniforms are std430
    // name types Uniform/Buffer/PushConstant?

    let layout_parts = ParsedLayoutParts::from_parts(&layout_parts).map_err(|x| {
        format!(
            "Error parsing binding type '{}' name '{}': {}",
            type_name, instance_name, x
        )
    })?;

    Ok(Some(ParseBindingResult {
        layout_parts,
        binding_type,
        type_name,
        fields,
        instance_name,
        array_sizes,
    }))
}

fn try_parse_const(code: &[char]) -> Result<Option<ParseConstantResult>, String> {
    let mut position = 0;

    //
    // See if it starts with const. If not, assume this isn't a constant and return None
    //
    if crate::parse_source::try_consume_literal(code, &mut position, "const").is_none() {
        return Ok(None);
    }

    crate::parse_source::skip_whitespace(code, &mut position);
    let type_name =
        crate::parse_source::try_consume_identifier(code, &mut position).ok_or(format!(
            "Expected type name while parsing constant:\n{}",
            crate::parse_source::characters_to_string(&code)
        ))?;

    crate::parse_source::skip_whitespace(code, &mut position);
    let instance_name =
        crate::parse_source::try_consume_identifier(code, &mut position).ok_or(format!(
            "Expected instance name while parsing constant:\n{}",
            crate::parse_source::characters_to_string(&code)
        ))?;

    crate::parse_source::skip_whitespace(code, &mut position);
    crate::parse_source::try_consume_literal(code, &mut position, "=").ok_or(format!(
        "Expected = while parsing constant:\n{}",
        crate::parse_source::characters_to_string(&code)
    ))?;

    crate::parse_source::skip_whitespace(code, &mut position);
    if crate::parse_source::is_string_at_position(code, position, "vec3")
        || crate::parse_source::is_string_at_position(code, position, "vec4")
        || crate::parse_source::is_string_at_position(code, position, "mat3")
        || crate::parse_source::is_string_at_position(code, position, "mat4")
    {
        // TODO(dvd): We don't support mat3, mat4, vec3, vec4.
        return Ok(Some(ParseConstantResult {
            type_name,
            instance_name,
            value: None,
        }));
    }

    // 1 or more values with + / - * between them.
    let mut values = Vec::default();
    loop {
        values.push(
            crate::parse_source::try_consume_constant_value(code, &mut position).ok_or(format!(
                "Expected value while parsing constant:\n{}",
                crate::parse_source::characters_to_string(&code)
            ))?,
        );

        if crate::parse_source::try_consume_literal(code, &mut position, ";").is_some() {
            break;
        } else {
            fn push_operator(
                values: &mut Vec<String>,
                operator: &str,
            ) {
                values.push(format!(" {} ", operator));
            }

            crate::parse_source::skip_whitespace(code, &mut position);
            if crate::parse_source::try_consume_literal(code, &mut position, "+").is_some() {
                push_operator(&mut values, "+");
            } else if crate::parse_source::try_consume_literal(code, &mut position, "-").is_some() {
                push_operator(&mut values, "-");
            } else if crate::parse_source::try_consume_literal(code, &mut position, "*").is_some() {
                push_operator(&mut values, "*");
            } else if crate::parse_source::try_consume_literal(code, &mut position, "/").is_some() {
                push_operator(&mut values, "/");
            } else {
                panic!(
                    "Expected ; while parsing constant:\n{}",
                    crate::parse_source::characters_to_string(&code)
                )
            }
            crate::parse_source::skip_whitespace(code, &mut position);
        }
    }

    Ok(Some(ParseConstantResult {
        type_name,
        instance_name,
        value: Some(values.join("")),
    }))
}

// fn generate_struct(result: &ParseStructResult, annotations: &StructAnnotations) -> Result<String, String> {
//     if !annotations.export.is_some() {
//         return Ok("".to_string())
//     }
//
// }

pub(crate) struct ParsedStructWithAnnotations {
    pub(crate) parsed: ParseStructResult,
    pub(crate) annotations: StructAnnotations,
}

#[derive(Debug)]
pub(crate) struct ParsedBindingWithAnnotations {
    pub(crate) parsed: ParseBindingResult,
    pub(crate) annotations: BindingAnnotations,
}

pub(crate) struct ParsedConstantWithAnnotations {
    pub(crate) parsed: ParseConstantResult,
}

pub(crate) struct ParseDeclarationsResult {
    pub(crate) structs: Vec<ParsedStructWithAnnotations>,
    pub(crate) bindings: Vec<ParsedBindingWithAnnotations>,
    pub(crate) constants: Vec<ParsedConstantWithAnnotations>,
}

pub(crate) fn parse_declarations(
    declarations: &[DeclarationText]
) -> Result<ParseDeclarationsResult, String> {
    let mut structs = Vec::default();
    let mut bindings = Vec::default();
    let mut constants = Vec::default();

    //
    // Parse all constants and their annotations
    //
    for declaration in declarations.iter().filter(|d| d.is_constant) {
        if let Some(const_result) = try_parse_const(&declaration.text)? {
            //
            // Handle const
            //
            //println!("Parsed a const {:?}", const_result);

            if !declaration.annotations.is_empty() {
                // TODO(dvd): Not sure what an annotation would imply for a const.
                return Err(format!(
                    "Annotations on consts not yet supported:\n{}",
                    crate::parse_source::characters_to_string(&declaration.text)
                ));
            }

            constants.push(ParsedConstantWithAnnotations {
                parsed: const_result,
            });
        } else {
            return Err(format!(
                "Annotations applied to declaration, but the declaration could not be parsed:\n{}",
                crate::parse_source::characters_to_string(&declaration.text)
            ));
        }
    }

    let parsed_constants = constants
        .iter()
        .map(|constant| constant.parsed.clone())
        .collect::<Vec<_>>();

    //
    // Parse all declarations and their annotations
    //
    for declaration in declarations.iter().filter(|d| !d.is_constant) {
        if let Some(struct_result) = try_parse_struct(&parsed_constants, &declaration.text)? {
            //
            // Handle struct
            //
            //println!("Parsed a struct {:?}", struct_result);

            let struct_annotations =
                StructAnnotations::new(&declaration.annotations).map_err(|e| {
                    format!(
                        "Failed to parse annotations for struct:\n\n{}\n\n{}",
                        crate::parse_source::characters_to_string(&declaration.text),
                        e,
                    )
                })?;

            structs.push(ParsedStructWithAnnotations {
                parsed: struct_result,
                annotations: struct_annotations,
            });
        } else if let Some(binding_result) =
            try_parse_binding(&parsed_constants, &declaration.text)?
        {
            //
            // Handle Binding
            //
            //println!("Parsed a binding {:?}", binding_result);

            let binding_annotations =
                BindingAnnotations::new(&declaration.annotations).map_err(|e| {
                    format!(
                        "Failed to parse annotations for binding:\n\n{}\n\n{}",
                        crate::parse_source::characters_to_string(&declaration.text),
                        e,
                    )
                })?;

            bindings.push(ParsedBindingWithAnnotations {
                parsed: binding_result,
                annotations: binding_annotations,
            });
        } else {
            return Err(format!(
                "Annotations applied to declaration, but the declaration could not be parsed:\n{}",
                crate::parse_source::characters_to_string(&declaration.text)
            ));
        }
    }

    Ok(ParseDeclarationsResult {
        structs,
        bindings,
        constants,
    })
}
