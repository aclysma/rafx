use std::error::Error;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

mod parse;
mod parse_declarations;

mod include;
use include::include_impl;
use include::IncludeType;
use log::LevelFilter;

mod codegen;

#[derive(StructOpt)]
pub struct ShaderProcessorArgs {
    //
    // For one file at a time
    //
    #[structopt(name = "glsl_file", long, parse(from_os_str))]
    pub glsl_file: Option<PathBuf>,
    #[structopt(name = "spv_file", long, parse(from_os_str))]
    pub spv_file: Option<PathBuf>,
    #[structopt(name = "rs_file", long, parse(from_os_str))]
    pub rs_file: Option<PathBuf>,

    //
    // For batch processing a folder
    //
    #[structopt(name = "glsl_path", long, parse(from_os_str))]
    pub glsl_files: Option<Vec<PathBuf>>,
    #[structopt(name = "spv_path", long, parse(from_os_str))]
    pub spv_path: Option<PathBuf>,
    #[structopt(name = "rs_path", long, parse(from_os_str))]
    pub rs_path: Option<PathBuf>,

    #[structopt(name = "shader_kind", long)]
    pub shader_kind: Option<String>,

    #[structopt(name = "trace", long)]
    pub trace: bool
}

// struct RustField {
//     name: String,
//     ty: String,
// }
//
// struct RustStruct {
//     name: String,
//     fields: Vec<RustField>,
// }

// struct ParseState {
//     code: String,
//     position: usize
// }

fn main() {
    let args = ShaderProcessorArgs::from_args();

    // Setup logging
    let level = if args.trace {
        LevelFilter::Trace
    } else {
        LevelFilter::Error
    };

    env_logger::Builder::from_default_env()
        .default_format_timestamp_nanos(true)
        .filter_level(level)
        .init();

    if let Err(e) = run(&args) {
        eprintln!("{}", e.to_string());
    }
}

#[derive(Debug)]
pub struct Declaration {
    pub text: Vec<char>,
    //comments: Vec<CommentText>
    pub annotations: Vec<Annotation>,
}

#[derive(Debug)]
pub struct Annotation {
    pub position: usize,
    pub text: Vec<char>,
}

fn handle_single_job(
    glsl_file: &Path,
    spv_file: Option<&PathBuf>,
    rs_file: Option<&PathBuf>,
    shader_kind: shaderc::ShaderKind
) -> Result<(), Box<dyn Error>> {
    log::trace!("read from file {:?}", glsl_file);
    let code = std::fs::read_to_string(&glsl_file)?;

    //
    // First, compile the code with shaderc. This will validate that it's well-formed.
    //
    let mut compiler = shaderc::Compiler::new().unwrap();
    let mut compile_options = shaderc::CompileOptions::new().unwrap();
    compile_options.set_include_callback(include::shaderc_include_callback);
    //compile_options.set_optimization_level(shaderc::OptimizationLevel::Performance);
    //compile_options.set_optimization_level(shaderc::OptimizationLevel::Size);
    //compile_options.set_generate_debug_info();

    //let file_name = args.glsl_file.file_name().unwrap();
    let entry_point_name = "main";
    let result = compiler.compile_into_spirv(
        &code,
        shader_kind,
        glsl_file.to_str().unwrap(),
        entry_point_name,
        Some(&compile_options),
    )?;

    //
    // Output the spv if desired
    //
    let spv_code = result.as_binary_u8();
    if let Some(spv_file) = &spv_file {
        std::fs::write(spv_file, spv_code)?;
    }

    let shader_module = spirv_reflect::ShaderModule::load_u8_data(spv_code)?;







/*
    println!("Bindings for {:?}:", glsl_file);
    let enumerated_entry_points = shader_module.enumerate_entry_points()?;
    for entry_point in enumerated_entry_points {
        for descriptor_set in &entry_point.descriptor_sets {
            for binding in &descriptor_set.bindings {
                println!("binding {} {} {}", binding.name, binding.set, binding.binding);
            }
        }
    }


    let asm_result = compiler.compile_into_spirv_assembly(&code, shader_kind, glsl_file.to_str().unwrap(), entry_point_name, Some(&compile_options))?;
    println!("{}", asm_result.as_text());
*/




    // let mut module = spirv_cross::spirv::Module::from_words(result.as_binary());
    // let mut ast = spirv_cross::spirv::Ast::<spirv_cross::hlsl::Target>::parse(&module)?;
    // println!("{}", ast.compile()?);

    //
    // Process the shader code
    //
    let parsed_source = parse::parse_shader_source(&glsl_file)?;
    let parsed_declarations = parse_declarations::parse_declarations(&parsed_source.declarations)?;
    let rust_code = codegen::generate_rust_code(&parsed_declarations, &shader_module)?;

    if let Some(rs_file) = &rs_file {
        std::fs::write(rs_file, rust_code)?;
    }

    Ok(())
}

fn run(args: &ShaderProcessorArgs) -> Result<(), Box<dyn Error>> {
    if let Some(glsl_file) = &args.glsl_file {
        let glsl_file_path: PathBuf = glsl_file.clone().into();

        let shader_kind = shader_kind_from_args(args)
            .or_else(|| deduce_default_shader_kind_from_path(glsl_file))
            .unwrap_or(shaderc::ShaderKind::InferFromSource);

        handle_single_job(
            &glsl_file_path,
            args.spv_file.as_ref(),
            args.rs_file.as_ref(),
            shader_kind
        )
    } else if let Some(glsl_files) = &args.glsl_files {
        let mut module_names = Vec::default();

        for glsl_file in glsl_files {
            //println!("glsl file: {:?}", glsl_file);

            for glob in glob::glob(glsl_file.to_str().unwrap())? {
                let glsl_file = glob?;
                log::trace!("path: {:?}", glsl_file);

                let file_name = glsl_file.file_name().ok_or_else(|| "Failed to get the filename from glob match".to_string())?.to_string_lossy();

                let module_name = file_name.to_string().to_lowercase().replace(".", "_");
                module_names.push(module_name.clone());
                let spv_name = format!("{}.spv", file_name);
                let rs_name = format!("{}.rs", module_name);

                let spv_path = args.spv_path.as_ref().map(|x| x.join(spv_name));
                let rs_path = args.rs_path.as_ref().map(|x| x.join(rs_name));

                log::trace!("glsl: {} spv: {:?} rs: {:?}", glsl_file.to_string_lossy(), spv_path, rs_path);

                let shader_kind = shader_kind_from_args(args)
                    .or_else(|| deduce_default_shader_kind_from_path(&glsl_file))
                    .unwrap_or(shaderc::ShaderKind::InferFromSource);

                handle_single_job(
                    &glsl_file,
                    spv_path.as_ref(),
                    rs_path.as_ref(),
                    shader_kind
                ).map_err(|x| format!("{}: {}", glsl_file.to_string_lossy(), x.to_string()))?;
            }
        }

        if let Some(rs_path) = &args.rs_path {
            let mut lib_file_string = String::default();
            for module_name in module_names {
                lib_file_string += &format!("pub mod {};\n", module_name);
            }

            //println!("{:?}", rs_path.join("lib.rs"))
            std::fs::write(rs_path.join("lib.rs"), lib_file_string)?;
        }

        Ok(())
    } else {
        Ok(())
    }
}

fn shader_kind_from_args(args: &ShaderProcessorArgs) -> Option<shaderc::ShaderKind> {
    let extensions = [
        ("vert", shaderc::ShaderKind::Vertex),
        ("frag", shaderc::ShaderKind::Fragment),
        ("tesc", shaderc::ShaderKind::TessControl),
        ("tese", shaderc::ShaderKind::TessEvaluation),
        ("geom", shaderc::ShaderKind::Geometry),
        ("comp", shaderc::ShaderKind::Compute),
        //("spvasm", shaderc::ShaderKind::Vertex), // we don't parse spvasm
        ("rgen", shaderc::ShaderKind::RayGeneration),
        ("rahit", shaderc::ShaderKind::AnyHit),
        ("rchit", shaderc::ShaderKind::ClosestHit),
        ("rmiss", shaderc::ShaderKind::Miss),
        ("rint", shaderc::ShaderKind::Intersection),
        ("rcall", shaderc::ShaderKind::Callable),
        ("task", shaderc::ShaderKind::Task),
        ("mesh", shaderc::ShaderKind::Mesh),
    ];

    if let Some(shader_kind) = &args.shader_kind {
        for &(extension, kind) in &extensions {
            if shader_kind == extension {
                return Some(kind);
            }
        }
    }

    None
}

// based on https://github.com/google/shaderc/blob/caa519ca532a6a3a0279509fce2ceb791c4f4651/glslc/src/shader_stage.cc#L69
fn deduce_default_shader_kind_from_path(path: &Path) -> Option<shaderc::ShaderKind> {
    let extensions = [
        ("vert", shaderc::ShaderKind::DefaultVertex),
        ("frag", shaderc::ShaderKind::DefaultFragment),
        ("tesc", shaderc::ShaderKind::DefaultTessControl),
        ("tese", shaderc::ShaderKind::DefaultTessEvaluation),
        ("geom", shaderc::ShaderKind::DefaultGeometry),
        ("comp", shaderc::ShaderKind::DefaultCompute),
        //("spvasm", shaderc::ShaderKind::Vertex), // we don't parse spvasm
        ("rgen", shaderc::ShaderKind::DefaultRayGeneration),
        ("rahit", shaderc::ShaderKind::DefaultAnyHit),
        ("rchit", shaderc::ShaderKind::DefaultClosestHit),
        ("rmiss", shaderc::ShaderKind::DefaultMiss),
        ("rint", shaderc::ShaderKind::DefaultIntersection),
        ("rcall", shaderc::ShaderKind::DefaultCallable),
        ("task", shaderc::ShaderKind::DefaultTask),
        ("mesh", shaderc::ShaderKind::DefaultMesh),
    ];

    if let Some(extension) = path.extension() {
        let as_str = extension.to_string_lossy();

        for &(extension, kind) in &extensions {
            if as_str.contains(extension) {
                return Some(kind);
            }
        }
    }

    None
}
