use std::error::Error;
use std::path::{PathBuf, Path};
use structopt::StructOpt;

mod parse;
mod include;
use include::IncludeType;
use include::include_impl;

#[derive(StructOpt)]
pub struct ShaderProcessorArgs {
    /// Path to the asset metadata database directory.
    #[structopt(name = "spv_file", long, parse(from_os_str))]
    pub spv_file: Option<PathBuf>,
    #[structopt(name = "glsl_file", long, parse(from_os_str))]
    pub glsl_file: PathBuf,
    // #[structopt(name = "rs_file", long, parse(from_os_str))]
    // pub rs_file: PathBuf,


    #[structopt(name = "shader_kind", long)]
    pub shader_kind: Option<String>,
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

fn run(args: &ShaderProcessorArgs) -> Result<(), Box<dyn Error>> {
    let glsl_file_path : PathBuf = args.glsl_file.clone().into();

    let shader_kind = shader_kind_from_args(args)
        .or_else(|| deduce_default_shader_kind_from_path(&args.glsl_file))
        .unwrap_or(shaderc::ShaderKind::InferFromSource);

    let code = std::fs::read_to_string(&glsl_file_path)?;

    //
    // First, compile the code with shaderc. This will validate that it's well-formed.
    //
    let mut compiler = shaderc::Compiler::new().unwrap();
    let mut compile_options = shaderc::CompileOptions::new().unwrap();
    compile_options.set_include_callback(include::shaderc_include_callback);

    //let file_name = args.glsl_file.file_name().unwrap();
    let entry_point_name = "main";
    let result = compiler.compile_into_spirv(
        &code,
        shader_kind,
        args.glsl_file.to_str().unwrap(),
        entry_point_name,
        Some(&compile_options),
    )?;

    //
    // Output the spv if desired
    //
    let spv_code = result.as_binary_u8();
    if let Some(spv_file) = &args.spv_file {
        std::fs::write(spv_file, spv_code)?;
    }

    //
    // Process the shader code
    //
    let parse_result = parse::parse_shader_source(&args.glsl_file)?;


    for declaration in parse_result.declarations {
        for annotation in &declaration.annotations {
            println!(">>    {}", parse::characters_to_string(&annotation.text[..]));
        }

        println!("{}\n", parse::characters_to_string(&declaration.text[..]));
    }

    // for declaration in declaration_ranges {
    //     let mut relevant_comments = Vec::default();
    //     while let Some(comment) = comments.front() {
    //         if comment.position < declaration.end {
    //             relevant_comments.push(comments.pop_front());
    //         }
    //     }
    // }

    Ok(())
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
        for &(extension, kind) in &extensions{
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