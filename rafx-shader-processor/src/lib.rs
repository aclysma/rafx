use std::error::Error;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

mod parse_source;
use parse_source::AnnotationText;
use parse_source::DeclarationText;

mod parse_declarations;

mod include;
use crate::parse_declarations::ParseDeclarationsResult;
use crate::parse_source::PreprocessorState;
use crate::reflect::ShaderProcessorRefectionData;
use crate::shader_types::{TypeAlignmentInfo, UserType};
use fnv::{FnvHashMap, FnvHashSet};
use include::include_impl;
use include::IncludeType;
use rafx_api::{
    RafxHashedShaderPackage, RafxShaderPackage, RafxShaderPackageDx12, RafxShaderPackageGles2,
    RafxShaderPackageGles3, RafxShaderPackageMetal, RafxShaderPackageVulkan,
};
use shaderc::{CompilationArtifact, Compiler, ShaderKind};
use spirv_cross::glsl::Target;
use spirv_cross::spirv::{Ast, ShaderResources};

mod codegen;

mod reflect;

mod shader_types;

const PREPROCESSOR_DEF_PLATFORM_RUST_CODEGEN: &'static str = "PLATFORM_RUST_CODEGEN";
const PREPROCESSOR_DEF_PLATFORM_DX12: &'static str = "PLATFORM_DX12";
const PREPROCESSOR_DEF_PLATFORM_VULKAN: &'static str = "PLATFORM_VULKAN";
const PREPROCESSOR_DEF_PLATFORM_METAL: &'static str = "PLATFORM_METAL";
const PREPROCESSOR_DEF_PLATFORM_GLES2: &'static str = "PLATFORM_GLES2";
const PREPROCESSOR_DEF_PLATFORM_GLES3: &'static str = "PLATFORM_GLES3";

#[derive(Clone, Copy, Debug)]
enum RsFileType {
    Lib,
    Mod,
}

#[derive(Debug)]
struct RsFileOption {
    path: PathBuf,
    file_type: RsFileType,
}

#[derive(StructOpt, Debug)]
pub struct ShaderProcessorArgs {
    //
    // For one file at a time
    //
    #[structopt(name = "glsl-file", long, parse(from_os_str))]
    pub glsl_file: Option<PathBuf>,
    #[structopt(name = "spv-file", long, parse(from_os_str))]
    pub spv_file: Option<PathBuf>,
    #[structopt(name = "rs-file", long, parse(from_os_str))]
    pub rs_file: Option<PathBuf>,
    #[structopt(name = "dx12-generated-src-file", long, parse(from_os_str))]
    pub dx12_generated_src_file: Option<PathBuf>,
    #[structopt(name = "metal-generated-src-file", long, parse(from_os_str))]
    pub metal_generated_src_file: Option<PathBuf>,
    #[structopt(name = "gles2-generated-src-file", long, parse(from_os_str))]
    pub gles2_generated_src_file: Option<PathBuf>,
    #[structopt(name = "gles3-generated-src-file", long, parse(from_os_str))]
    pub gles3_generated_src_file: Option<PathBuf>,
    #[structopt(name = "cooked-shader-file", long, parse(from_os_str))]
    pub cooked_shader_file: Option<PathBuf>,

    //
    // For batch processing a folder
    //
    #[structopt(name = "glsl-path", long, parse(from_os_str))]
    pub glsl_files: Option<PathBuf>,
    #[structopt(name = "spv-path", long, parse(from_os_str))]
    pub spv_path: Option<PathBuf>,
    #[structopt(name = "rs-lib-path", long, parse(from_os_str))]
    pub rs_lib_path: Option<PathBuf>,
    #[structopt(name = "rs-mod-path", long, parse(from_os_str))]
    pub rs_mod_path: Option<PathBuf>,
    #[structopt(name = "dx12-generated-src-path", long, parse(from_os_str))]
    pub dx12_generated_src_path: Option<PathBuf>,
    #[structopt(name = "metal-generated-src-path", long, parse(from_os_str))]
    pub metal_generated_src_path: Option<PathBuf>,
    #[structopt(name = "gles2-generated-src-path", long, parse(from_os_str))]
    pub gles2_generated_src_path: Option<PathBuf>,
    #[structopt(name = "gles3-generated-src-path", long, parse(from_os_str))]
    pub gles3_generated_src_path: Option<PathBuf>,
    #[structopt(name = "cooked-shaders-path", long, parse(from_os_str))]
    pub cooked_shaders_path: Option<PathBuf>,

    #[structopt(name = "shader-kind", long)]
    pub shader_kind: Option<String>,

    #[structopt(name = "trace", long)]
    pub trace: bool,

    #[structopt(name = "optimize-shaders", long)]
    pub optimize_shaders: bool,

    #[structopt(name = "package-vk", long)]
    pub package_vk: bool,
    #[structopt(name = "package-dx12", long)]
    pub package_dx12: bool,
    #[structopt(name = "package-metal", long)]
    pub package_metal: bool,
    #[structopt(name = "package-gles2", long)]
    pub package_gles2: bool,
    #[structopt(name = "package-gles3", long)]
    pub package_gles3: bool,
    #[structopt(name = "package-all", long)]
    pub package_all: bool,

    #[structopt(name = "for-rafx-framework-crate", long)]
    pub for_rafx_framework_crate: bool,
}

pub fn run(args: &ShaderProcessorArgs) -> Result<(), Box<dyn Error>> {
    log::trace!("Shader processor args: {:#?}", args);
    if args.rs_lib_path.is_some() && args.rs_mod_path.is_some() {
        Err("Both --rs-lib-path and --rs-mod-path were provided, using both at the same time is not supported.")?;
    }

    let rs_file_option = if let Some(path) = &args.rs_lib_path {
        Some(RsFileOption {
            path: path.clone(),
            file_type: RsFileType::Lib,
        })
    } else if let Some(path) = &args.rs_mod_path {
        Some(RsFileOption {
            path: path.clone(),
            file_type: RsFileType::Mod,
        })
    } else {
        None
    };

    if let Some(glsl_file) = &args.glsl_file {
        //
        // Handle a single file given via --glsl_file. In this mode, the output files are explicit
        //
        log::info!("Processing file {:?}", glsl_file);

        //
        // Try to determine what kind of shader this is from the file name
        //
        let shader_kind = shader_kind_from_args(args)
            .or_else(|| deduce_default_shader_kind_from_path(glsl_file))
            .unwrap_or(shaderc::ShaderKind::InferFromSource);

        //
        // Process this shader and write to output files
        //
        process_glsl_shader(
            glsl_file,
            args.spv_file.as_ref(),
            &rs_file_option,
            args.dx12_generated_src_path.as_ref(),
            args.metal_generated_src_file.as_ref(),
            args.gles2_generated_src_file.as_ref(),
            args.gles3_generated_src_file.as_ref(),
            args.cooked_shader_file.as_ref(),
            shader_kind,
            &args,
        )
        .map_err(|x| format!("{}: {}", glsl_file.to_string_lossy(), x.to_string()))?;

        Ok(())
    } else if let Some(glsl_files) = &args.glsl_files {
        log::trace!("glsl files {:?}", args.glsl_files);
        process_directory(glsl_files, &args, &rs_file_option)
    } else {
        Ok(())
    }
}

//
// Handle a batch of file patterns (such as *.frag) via --glsl_files. Infer output files
// based on other args given in the form of output directories
//
fn process_directory(
    glsl_files: &PathBuf,
    args: &ShaderProcessorArgs,
    rs_file_option: &Option<RsFileOption>,
) -> Result<(), Box<dyn Error>> {
    // This will accumulate rust module names so we can produce a lib.rs if needed
    let mut module_names = FnvHashMap::<PathBuf, FnvHashSet<String>>::default();

    log::trace!("GLSL Root Dir: {:?}", glsl_files);

    let glob_walker = globwalk::GlobWalkerBuilder::from_patterns(
        glsl_files.to_str().unwrap(),
        &["*.{vert,frag,comp}"],
    )
    .file_type(globwalk::FileType::FILE)
    .build()?;

    for glob in glob_walker {
        //
        // Determine the files we will write out
        //
        let glsl_file = glob?;
        log::info!("Processing file {:?}", glsl_file.path());

        let file_name = glsl_file.file_name().to_string_lossy();

        let empty_path = PathBuf::new();
        let outfile_prefix = glsl_file
            .path()
            .strip_prefix(glsl_files)?
            .parent()
            .unwrap_or(&empty_path);

        let rs_module_name = file_name.to_string().to_lowercase().replace(".", "_");
        let rs_name = format!("{}.rs", rs_module_name);
        let rs_file_option = rs_file_option.as_ref().map(|x| RsFileOption {
            path: x.path.join(outfile_prefix).join(rs_name),
            file_type: x.file_type,
        });

        let spv_name = format!("{}.spv", file_name);
        let spv_path = args
            .spv_path
            .as_ref()
            .map(|x| x.join(outfile_prefix).join(spv_name));

        let dx12_src_name = format!("{}.hlsl", file_name);
        let dx12_generated_src_path = args
            .dx12_generated_src_path
            .as_ref()
            .map(|x| x.join(outfile_prefix).join(dx12_src_name));

        let metal_src_name = format!("{}.metal", file_name);
        let metal_generated_src_path = args
            .metal_generated_src_path
            .as_ref()
            .map(|x| x.join(outfile_prefix).join(metal_src_name));

        let gles2_src_name = format!("{}.gles2", file_name);
        let gles2_generated_src_path = args
            .gles2_generated_src_path
            .as_ref()
            .map(|x| x.join(outfile_prefix).join(gles2_src_name));

        let gles3_src_name = format!("{}.gles3", file_name);
        let gles3_generated_src_path = args
            .gles3_generated_src_path
            .as_ref()
            .map(|x| x.join(outfile_prefix).join(gles3_src_name));

        let cooked_shader_name = format!("{}.cookedshaderpackage", file_name);
        let cooked_shader_path = args
            .cooked_shaders_path
            .as_ref()
            .map(|x| x.join(outfile_prefix).join(cooked_shader_name));

        //
        // Try to determine what kind of shader this is from the file name
        //
        let shader_kind = shader_kind_from_args(args)
            .or_else(|| deduce_default_shader_kind_from_path(glsl_file.path()))
            .unwrap_or(shaderc::ShaderKind::InferFromSource);

        //
        // Process this shader and write to output files
        //
        process_glsl_shader(
            glsl_file.path(),
            spv_path.as_ref(),
            &rs_file_option,
            dx12_generated_src_path.as_ref(),
            metal_generated_src_path.as_ref(),
            gles2_generated_src_path.as_ref(),
            gles3_generated_src_path.as_ref(),
            cooked_shader_path.as_ref(),
            shader_kind,
            &args,
        )
        .map_err(|x| format!("{}: {}", glsl_file.path().to_string_lossy(), x.to_string()))?;

        //
        // Add the module name to this list so we can generate a lib.rs later
        //
        if rs_file_option.is_some() {
            let module_names = module_names
                .entry(outfile_prefix.to_path_buf())
                .or_default();
            module_names.insert(rs_module_name.clone());
        }
    }
    //
    // Generate lib.rs or mod.rs files that includes all the compiled shaders
    //
    if let Some(rs_path) = &rs_file_option {
        // First ensure that for any nested submodules, they are declared in lib.rs/mod.rs files in
        // the parent dirs
        let outfile_prefixes: Vec<_> = module_names.keys().cloned().collect();
        for mut outfile_prefix in outfile_prefixes {
            while let Some(parent) = outfile_prefix.parent() {
                let new_module_name = outfile_prefix
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string();

                log::trace!("add module {:?} to {:?}", new_module_name, parent);

                let module_names = module_names.entry(parent.to_path_buf()).or_default();
                module_names.insert(new_module_name);

                outfile_prefix = parent.to_path_buf();
            }
        }

        // Generate all lib.rs/mod.rs files
        for (outfile_prefix, module_names) in module_names {
            let module_filename = match rs_path.file_type {
                RsFileType::Lib => "lib.rs",
                RsFileType::Mod => "mod.rs",
            };
            let lib_file_path = rs_path.path.join(outfile_prefix).join(module_filename);
            log::trace!("Write lib/mod file {:?} {:?}", lib_file_path, module_names);

            let mut lib_file_string = String::default();
            lib_file_string += "// This code is auto-generated by the shader processor.\n\n";
            lib_file_string += "#![allow(dead_code)]\n\n";

            for module_name in module_names {
                lib_file_string += &format!("pub mod {};\n", module_name);
            }

            write_output_file(&lib_file_path, lib_file_string)?;
        }
    }

    Ok(())
}

fn process_glsl_shader(
    glsl_file: &Path,
    spv_file: Option<&PathBuf>,
    rs_file: &Option<RsFileOption>,
    dx12_generated_src_file: Option<&PathBuf>,
    metal_generated_src_file: Option<&PathBuf>,
    gles2_generated_src_file: Option<&PathBuf>,
    gles3_generated_src_file: Option<&PathBuf>,
    cooked_shader_file: Option<&PathBuf>,
    shader_kind: shaderc::ShaderKind,
    args: &ShaderProcessorArgs,
) -> Result<(), Box<dyn Error>> {
    log::trace!("--- Start processing shader job ---");
    log::trace!("glsl: {:?}", glsl_file);
    log::trace!("spv: {:?}", spv_file);
    log::trace!("rs: {:?}", rs_file);
    log::trace!("dx12: {:?}", dx12_generated_src_file);
    log::trace!("metal: {:?}", metal_generated_src_file);
    log::trace!("gles2: {:?}", gles2_generated_src_file);
    log::trace!("gles3: {:?}", gles3_generated_src_file);
    log::trace!("cooked: {:?}", cooked_shader_file);
    log::trace!("shader kind: {:?}", shader_kind);

    let package_vk = (args.package_all || args.package_vk) && cooked_shader_file.is_some();
    let package_dx12 = (args.package_all || args.package_dx12) && cooked_shader_file.is_some();
    let package_metal = (args.package_all || args.package_metal) && cooked_shader_file.is_some();
    let package_gles2 = (args.package_all || args.package_gles2) && cooked_shader_file.is_some();
    let package_gles3 = (args.package_all || args.package_gles3) && cooked_shader_file.is_some();

    log::trace!(
        "package VK: {} dx12: {} Metal: {} GLES2: {} GLES3: {}",
        package_vk,
        package_dx12,
        package_metal,
        package_gles2,
        package_gles3
    );

    if cooked_shader_file.is_some()
        && !(package_vk || package_dx12 || package_metal || package_gles2 || package_gles3)
    {
        Err("A cooked shader file or path was specified but no shader types are specified to package. Pass --package-vk, --package-dx12, --package-metal, --package-gles2, --package-gles3, or --package-all")?;
    }

    let code = std::fs::read_to_string(&glsl_file)?;
    let entry_point_name = "main";

    //
    // First, compile the code with shaderc. This will validate that it's well-formed. We will also
    // use the produced spv to create reflection data. This first pass must be UNOPTIMIZED so that
    // we don't drop reflection data for unused elements.
    //
    // We want to preserve unused fields so that the rust API we generate does not substantially
    // change and cause spurious compile errors just because a line of code gets commented out in
    // the shader. (In the future we may want to generate the API but make it a noop.)
    //
    let generate_reflection_data = rs_file.is_some()
        || cooked_shader_file.is_some()
        || dx12_generated_src_file.is_some()
        || metal_generated_src_file.is_some()
        || gles2_generated_src_file.is_some();

    let require_semantics = cooked_shader_file.is_some() || dx12_generated_src_file.is_some();
    let compiler = shaderc::Compiler::new().unwrap();

    let compile_parameters = CompileParameters {
        glsl_file,
        shader_kind,
        code: &code,
        entry_point_name,
        generate_reflection_data,
        require_semantics,
        compiler: &compiler,
    };

    let rust_code = if rs_file.is_some() {
        let mut compile_result =
            compile_glsl(&compile_parameters, PREPROCESSOR_DEF_PLATFORM_RUST_CODEGEN)?;

        log::trace!("{:?}: generate rust code", glsl_file);
        let reflected_entry_point = compile_result
            .reflection_data
            .as_ref()
            .unwrap()
            .reflection
            .iter()
            .find(|x| x.rafx_api_reflection.entry_point_name == entry_point_name)
            .ok_or_else(|| {
                format!(
                    "Could not find entry point {} in compiled shader file",
                    entry_point_name
                )
            })?;

        //
        // Generate rust code that matches up with the shader
        //
        log::trace!("{:?}: generate rust code", glsl_file);
        Some(codegen::generate_rust_code(
            &compile_result.builtin_types,
            &mut compile_result.user_types,
            &compile_result.parsed_declarations,
            //&spirv_reflect_module,
            &reflected_entry_point,
            args.for_rafx_framework_crate,
        )?)
    } else {
        None
    };

    let vk_output = if spv_file.is_some() || package_vk {
        Some(cross_compile_to_vulkan(
            glsl_file,
            &compile_parameters,
            &args,
        )?)
    } else {
        None
    };

    let dx12_output = if dx12_generated_src_file.is_some() || package_dx12 {
        Some(cross_compile_to_dx12(glsl_file, &compile_parameters)?)
    } else {
        None
    };

    let metal_output = if metal_generated_src_file.is_some() || package_metal {
        Some(cross_compile_to_metal(glsl_file, &compile_parameters)?)
    } else {
        None
    };

    let gles2_output = if gles2_generated_src_file.is_some() || package_gles2 {
        Some(cross_compile_to_gles2(glsl_file, &compile_parameters)?)
    } else {
        None
    };

    let gles3_output = if gles3_generated_src_file.is_some() || package_gles3 {
        Some(cross_compile_to_gles3(glsl_file, &compile_parameters)?)
    } else {
        None
    };

    //
    // Write out the spv and rust files if desired
    //
    if let Some(spv_file) = &spv_file {
        write_output_file(spv_file, &vk_output.as_ref().unwrap().vk_spv)?;
    }

    if let Some(rs_file) = &rs_file {
        write_output_file(&rs_file.path, rust_code.unwrap())?;
    }

    if let Some(dx12_generated_src_file) = &dx12_generated_src_file {
        write_output_file(
            dx12_generated_src_file,
            &dx12_output.as_ref().unwrap().dx12_src,
        )?;
    }

    if let Some(metal_generated_src_file) = &metal_generated_src_file {
        write_output_file(
            metal_generated_src_file,
            &metal_output.as_ref().unwrap().metal_src,
        )?;
    }

    if let Some(gles2_generated_src_file) = &gles2_generated_src_file {
        write_output_file(
            gles2_generated_src_file,
            &gles2_output.as_ref().unwrap().gles2_src,
        )?;
    }

    if let Some(gles3_generated_src_file) = &gles3_generated_src_file {
        write_output_file(
            gles3_generated_src_file,
            &gles3_output.as_ref().unwrap().gles3_src,
        )?;
    }

    // Don't worry about the return value
    log::trace!("{:?}: cook shader", glsl_file);
    let cooked_shader = if cooked_shader_file.is_some() {
        let mut shader_package = RafxShaderPackage::default();

        if package_vk {
            let vk_output = vk_output.unwrap();
            shader_package.vk = Some(RafxShaderPackageVulkan::SpvBytes(vk_output.vk_spv));
            shader_package.vk_reflection = vk_output.reflection_data.map(|x| x.reflection);
        };

        if package_dx12 {
            let dx12_output = dx12_output.unwrap();
            shader_package.dx12 = Some(RafxShaderPackageDx12::Src(dx12_output.dx12_src));
            shader_package.dx12_reflection = dx12_output.reflection_data.map(|x| x.reflection);
        };

        if package_metal {
            let metal_output = metal_output.unwrap();
            shader_package.metal = Some(RafxShaderPackageMetal::Src(metal_output.metal_src));
            shader_package.metal_reflection = metal_output.reflection_data.map(|x| x.reflection);
        };

        if package_gles2 {
            let gles2_output = gles2_output.unwrap();
            shader_package.gles2 = Some(RafxShaderPackageGles2::Src(gles2_output.gles2_src));
            shader_package.gles2_reflection = gles2_output.reflection_data.map(|x| x.reflection);
        };

        if package_gles3 {
            let gles3_output = gles3_output.unwrap();
            shader_package.gles3 = Some(RafxShaderPackageGles3::Src(gles3_output.gles3_src));
            shader_package.gles3_reflection = gles3_output.reflection_data.map(|x| x.reflection);
        };

        shader_package.debug_name =
            Some(glsl_file.file_name().unwrap().to_string_lossy().to_string());
        let hashed_shader_package = RafxHashedShaderPackage::new(shader_package);

        let serialized = bincode::serialize(&hashed_shader_package)
            .map_err(|x| format!("Failed to serialize cooked shader: {}", x))?;
        Some(serialized)
    } else {
        None
    };

    if let Some(cooked_shader_file) = &cooked_shader_file {
        write_output_file(cooked_shader_file, cooked_shader.unwrap())?;
    }

    Ok(())
}

struct CompileParameters<'a> {
    glsl_file: &'a Path,
    shader_kind: ShaderKind,
    code: &'a str,
    entry_point_name: &'a str,
    generate_reflection_data: bool,
    require_semantics: bool,
    compiler: &'a Compiler,
}

struct CompileResult {
    unoptimized_spv: CompilationArtifact,
    parsed_declarations: ParseDeclarationsResult,
    ast: Ast<Target>,
    user_types: FnvHashMap<String, UserType>,
    builtin_types: FnvHashMap<String, TypeAlignmentInfo>,
    reflection_data: Option<ShaderProcessorRefectionData>,
}

fn try_load_override_src(
    parameters: &CompileParameters,
    original_path: &Path,
    extension: &str,
    platform_define: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    let mut override_path = original_path.as_os_str().to_os_string();
    override_path.push(extension);
    let override_path = PathBuf::from(override_path);
    if override_path.exists() {
        log::info!(
            "  Override shader {:?} with {:?}",
            original_path,
            override_path.to_string_lossy()
        );

        let override_src = std::fs::read_to_string(&override_path)?;

        // We want to inline all the #includes because we are packaging the source for compilation
        // on target hardware and it won't be able to #include dependencies.
        let preprocessed_src =
            parse_source::inline_includes_in_override_src(&override_path, &override_src)?;

        Ok(Some(preprocessed_src))
    } else {
        Ok(None)
    }
}

fn compile_glsl(
    parameters: &CompileParameters,
    platform_define: &str,
) -> Result<CompileResult, Box<dyn Error>> {
    log::trace!("{:?}: compile unoptimized", parameters.glsl_file);
    let (unoptimized_spv, parsed_source) = {
        let mut compile_options = shaderc::CompileOptions::new().unwrap();
        compile_options.set_include_callback(include::shaderc_include_callback);
        compile_options.set_generate_debug_info();
        compile_options.add_macro_definition(platform_define, Some("1"));

        log::trace!("compile to spriv for platform {:?}", platform_define);

        let unoptimized_spv = parameters.compiler.compile_into_spirv(
            &parameters.code,
            parameters.shader_kind,
            parameters.glsl_file.to_str().unwrap(),
            parameters.entry_point_name,
            Some(&compile_options),
        )?;

        log::trace!("{:?}: parse glsl", parameters.glsl_file);

        let mut preprocessor_state = PreprocessorState::default();
        preprocessor_state.add_define(platform_define.to_string(), "1".to_string());
        let parsed_source = parse_source::parse_glsl_src(
            &parameters.glsl_file,
            &parameters.code,
            &mut preprocessor_state,
        )?;

        (unoptimized_spv, parsed_source)
    };

    //
    // Read the unoptimized spv into spirv_cross so that we can grab reflection data
    //
    log::trace!("{:?}: read spirv_cross module", parameters.glsl_file);
    let spirv_cross_module = spirv_cross::spirv::Module::from_words(unoptimized_spv.as_binary());

    //
    // Parse the declarations that were extracted from the source file
    //
    log::trace!("{:?}: parse declarations", parameters.glsl_file);
    let parsed_declarations = parse_declarations::parse_declarations(&parsed_source.declarations)?;
    let is_compute_shader = normalize_shader_kind(parameters.shader_kind) == ShaderKind::Compute;
    if parsed_declarations.group_size.is_some() && !is_compute_shader {
        Err("The shader is not a compute shader but a group size was specified")?;
    } else if parsed_declarations.group_size.is_none() && is_compute_shader {
        Err("The shader is a compute shader but a group size was not specified. Expected to find something like `layout(local_size_x = 1, local_size_y = 1, local_size_z = 1) in;` in the shader")?;
    }

    log::trace!("{:?}: generate spirv_cross ast", parameters.glsl_file);
    let mut spirv_cross_glsl_options = spirv_cross::glsl::CompilerOptions::default();
    spirv_cross_glsl_options.vulkan_semantics = true;
    let mut ast = spirv_cross::spirv::Ast::<spirv_cross::glsl::Target>::parse(&spirv_cross_module)?;
    ast.set_compiler_options(&spirv_cross_glsl_options)?;

    log::trace!("{:?}: generate shader types", parameters.glsl_file);
    let user_types = shader_types::create_user_type_lookup(&parsed_declarations)?;
    let builtin_types = shader_types::create_builtin_type_lookup();

    let reflected_data = if parameters.generate_reflection_data {
        log::trace!("{:?}: generate reflection data", parameters.glsl_file);
        Some(reflect::reflect_data(
            &builtin_types,
            &user_types,
            &ast,
            &parsed_declarations,
            parameters.require_semantics,
        )?)
    } else {
        None
    };

    Ok(CompileResult {
        unoptimized_spv,
        parsed_declarations,
        ast,
        user_types,
        builtin_types,
        reflection_data: reflected_data,
    })
}

pub struct CrossCompileOutputVulkan {
    vk_spv: Vec<u8>,
    reflection_data: Option<ShaderProcessorRefectionData>,
}

fn cross_compile_to_vulkan(
    glsl_file: &Path,
    compile_parameters: &CompileParameters,
    args: &ShaderProcessorArgs,
) -> Result<CrossCompileOutputVulkan, Box<dyn Error>> {
    log::trace!("{:?}: create vulkan", glsl_file);
    let compile_result = compile_glsl(compile_parameters, PREPROCESSOR_DEF_PLATFORM_VULKAN)?;

    let vk_spv = if args.optimize_shaders {
        let mut compile_options = shaderc::CompileOptions::new().unwrap();
        compile_options.set_include_callback(include::shaderc_include_callback);
        compile_options.set_optimization_level(shaderc::OptimizationLevel::Performance);
        //NOTE: Could also use shaderc::OptimizationLevel::Size

        compile_parameters
            .compiler
            .compile_into_spirv(
                compile_parameters.code,
                compile_parameters.shader_kind,
                glsl_file.to_str().unwrap(),
                compile_parameters.entry_point_name,
                Some(&compile_options),
            )?
            .as_binary_u8()
            .to_vec()
    } else {
        compile_result.unoptimized_spv.as_binary_u8().to_vec()
    };

    Ok(CrossCompileOutputVulkan {
        vk_spv,
        reflection_data: compile_result.reflection_data,
    })
}

pub struct CrossCompileOutputDx12 {
    dx12_src: String,
    reflection_data: Option<ShaderProcessorRefectionData>,
}

fn cross_compile_to_dx12(
    glsl_file: &Path,
    compile_parameters: &CompileParameters,
) -> Result<CrossCompileOutputDx12, Box<dyn Error>> {
    log::trace!("{:?}: create dx12", glsl_file);
    let compile_result = compile_glsl(compile_parameters, PREPROCESSOR_DEF_PLATFORM_DX12)?;

    let dx12_src = if let Some(src) = try_load_override_src(
        compile_parameters,
        glsl_file,
        ".hlsl",
        PREPROCESSOR_DEF_PLATFORM_DX12,
    )? {
        src
    } else {
        let spirv_cross_module =
            spirv_cross::spirv::Module::from_words(compile_result.unoptimized_spv.as_binary());

        let mut hlsl_ast =
            spirv_cross::spirv::Ast::<spirv_cross::hlsl::Target>::parse(&spirv_cross_module)?;
        let mut spirv_cross_hlsl_options = spirv_cross::hlsl::CompilerOptions::default();
        spirv_cross_hlsl_options.shader_model = spirv_cross::hlsl::ShaderModel::V6_0;
        spirv_cross_hlsl_options.flatten_matrix_vertex_input_semantics = true;
        spirv_cross_hlsl_options.force_storage_buffer_as_uav = true;

        for assignment in &compile_result
            .reflection_data
            .as_ref()
            .unwrap()
            .hlsl_register_assignments
        {
            hlsl_ast.add_resource_binding(assignment)?;
        }

        for remap in &compile_result
            .reflection_data
            .as_ref()
            .unwrap()
            .hlsl_vertex_attribute_remaps
        {
            // We require semantics to produce HLSL, an error should be thrown earlier if they are missing
            assert!(!remap.semantic.is_empty());
            if !remap.semantic.is_empty() {
                hlsl_ast.add_vertex_attribute_remap(remap)?;
            }
        }

        hlsl_ast.set_compiler_options(&spirv_cross_hlsl_options)?;

        hlsl_ast.compile()?
    };

    Ok(CrossCompileOutputDx12 {
        dx12_src,
        reflection_data: compile_result.reflection_data,
    })
}

pub struct CrossCompileOutputMetal {
    metal_src: String,
    reflection_data: Option<ShaderProcessorRefectionData>,
}

fn cross_compile_to_metal(
    glsl_file: &Path,
    compile_parameters: &CompileParameters,
) -> Result<CrossCompileOutputMetal, Box<dyn Error>> {
    log::trace!("{:?}: create msl", glsl_file);
    let compile_result = compile_glsl(compile_parameters, PREPROCESSOR_DEF_PLATFORM_METAL)?;

    let metal_src = if let Some(src) = try_load_override_src(
        compile_parameters,
        glsl_file,
        ".metal",
        PREPROCESSOR_DEF_PLATFORM_METAL,
    )? {
        src
    } else {
        let spirv_cross_module =
            spirv_cross::spirv::Module::from_words(compile_result.unoptimized_spv.as_binary());

        let mut msl_ast =
            spirv_cross::spirv::Ast::<spirv_cross::msl::Target>::parse(&spirv_cross_module)?;
        let mut spirv_cross_msl_options = spirv_cross::msl::CompilerOptions::default();
        spirv_cross_msl_options.version = spirv_cross::msl::Version::V2_1;
        spirv_cross_msl_options.enable_argument_buffers = true;
        spirv_cross_msl_options.force_active_argument_buffer_resources = true;
        //TODO: Add equivalent to --msl-no-clip-distance-user-varying

        //TODO: Set this up
        spirv_cross_msl_options.resource_binding_overrides = compile_result
            .reflection_data
            .as_ref()
            .unwrap()
            .msl_argument_buffer_assignments
            .clone();
        //println!(" binding overrides {:?}", spirv_cross_msl_options.resource_binding_overrides);
        //spirv_cross_msl_options.vertex_attribute_overrides
        spirv_cross_msl_options.const_samplers = compile_result
            .reflection_data
            .as_ref()
            .unwrap()
            .msl_const_samplers
            .clone();

        msl_ast.set_compiler_options(&spirv_cross_msl_options)?;
        msl_ast.compile()?
    };

    Ok(CrossCompileOutputMetal {
        metal_src,
        reflection_data: compile_result.reflection_data,
    })
}

pub struct CrossCompileOutputGles3 {
    gles3_src: String,
    reflection_data: Option<ShaderProcessorRefectionData>,
}

fn cross_compile_to_gles3(
    glsl_file: &Path,
    compile_parameters: &CompileParameters,
) -> Result<CrossCompileOutputGles3, Box<dyn Error>> {
    log::trace!("{:?}: create gles3", glsl_file);
    let mut compile_result = compile_glsl(compile_parameters, PREPROCESSOR_DEF_PLATFORM_GLES3)?;

    let gles3_src = if let Some(src) = try_load_override_src(
        compile_parameters,
        glsl_file,
        ".gles3",
        PREPROCESSOR_DEF_PLATFORM_GLES3,
    )? {
        src
    } else {
        let spirv_cross_module =
            spirv_cross::spirv::Module::from_words(compile_result.unoptimized_spv.as_binary());

        let mut gles3_ast =
            spirv_cross::spirv::Ast::<spirv_cross::glsl::Target>::parse(&spirv_cross_module)?;
        let mut spirv_cross_gles3_options = spirv_cross::glsl::CompilerOptions::default();
        spirv_cross_gles3_options.version = spirv_cross::glsl::Version::V3_00Es;
        spirv_cross_gles3_options.vulkan_semantics = false;
        spirv_cross_gles3_options.vertex.transform_clip_space = true;
        spirv_cross_gles3_options.vertex.invert_y = true;

        let shader_resources = compile_result.ast.get_shader_resources()?;

        rename_gl_samplers(&mut compile_result.reflection_data, &mut gles3_ast)?;
        rename_gl_in_out_attributes(
            compile_parameters.shader_kind,
            &mut gles3_ast,
            &shader_resources,
        )?;

        gles3_ast.set_compiler_options(&spirv_cross_gles3_options)?;
        gles3_ast.compile()?
    };

    Ok(CrossCompileOutputGles3 {
        gles3_src,
        reflection_data: compile_result.reflection_data,
    })
}

pub struct CrossCompileOutputGles2 {
    gles2_src: String,
    reflection_data: Option<ShaderProcessorRefectionData>,
}

fn cross_compile_to_gles2(
    glsl_file: &Path,
    compile_parameters: &CompileParameters,
) -> Result<CrossCompileOutputGles2, Box<dyn Error>> {
    log::trace!("{:?}: create gles2", glsl_file);
    let mut compile_result = compile_glsl(compile_parameters, PREPROCESSOR_DEF_PLATFORM_GLES2)?;

    let gles2_src = if let Some(src) = try_load_override_src(
        compile_parameters,
        glsl_file,
        ".gles2",
        PREPROCESSOR_DEF_PLATFORM_GLES2,
    )? {
        src
    } else {
        let spirv_cross_module =
            spirv_cross::spirv::Module::from_words(compile_result.unoptimized_spv.as_binary());

        let mut gles2_ast =
            spirv_cross::spirv::Ast::<spirv_cross::glsl::Target>::parse(&spirv_cross_module)?;
        let mut spirv_cross_gles2_options = spirv_cross::glsl::CompilerOptions::default();
        spirv_cross_gles2_options.version = spirv_cross::glsl::Version::V1_00Es;
        spirv_cross_gles2_options.vulkan_semantics = false;
        spirv_cross_gles2_options.vertex.transform_clip_space = true;
        spirv_cross_gles2_options.vertex.invert_y = true;

        let shader_resources = compile_result.ast.get_shader_resources()?;

        // Rename uniform blocks to be consistent with how they would appear in GL ES 3.0. This way
        // we can consistently use the same GL name across both backends
        for resource in &shader_resources.uniform_buffers {
            let block_name = gles2_ast.get_name(resource.base_type_id)?;
            gles2_ast.set_name(
                resource.base_type_id,
                &format!("{}_UniformBlock", block_name),
            )?;
            gles2_ast.set_name(resource.id, &block_name)?;
        }

        rename_gl_samplers(&mut compile_result.reflection_data, &mut gles2_ast)?;
        rename_gl_in_out_attributes(
            compile_parameters.shader_kind,
            &mut gles2_ast,
            &shader_resources,
        )?;

        gles2_ast.set_compiler_options(&spirv_cross_gles2_options)?;
        gles2_ast.compile()?
    };

    Ok(CrossCompileOutputGles2 {
        gles2_src,
        reflection_data: compile_result.reflection_data,
    })
}

fn write_output_file<C: AsRef<[u8]>>(
    path: &PathBuf,
    contents: C,
) -> std::io::Result<()> {
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(path, contents)
}

fn rename_gl_samplers(
    reflected_data: &mut Option<ShaderProcessorRefectionData>,
    ast: &mut Ast<Target>,
) -> Result<(), Box<dyn Error>> {
    ast.build_combined_image_samplers()?;

    let mut all_combined_textures = FnvHashSet::default();
    for remap in ast.get_combined_image_samplers()? {
        let texture_name = ast.get_name(remap.image_id)?;
        let sampler_name = ast.get_name(remap.sampler_id)?;

        let already_sampled = !all_combined_textures.insert(remap.image_id);
        if already_sampled {
            Err(format!("The texture {} is being read by multiple samplers. This is not supported in GL ES 2.0", texture_name))?;
        }

        if let Some(reflected_data) = reflected_data {
            reflected_data.set_gl_sampler_name(&texture_name, &sampler_name);
        }

        ast.set_name(remap.combined_id, &texture_name)?
    }

    Ok(())
}

fn rename_gl_in_out_attributes(
    shader_kind: ShaderKind,
    ast: &mut Ast<Target>,
    shader_resources: &ShaderResources,
) -> Result<(), Box<dyn Error>> {
    if normalize_shader_kind(shader_kind) == ShaderKind::Vertex {
        for resource in &shader_resources.stage_outputs {
            let location =
                ast.get_decoration(resource.id, spirv_cross::spirv::Decoration::Location)?;
            ast.rename_interface_variable(
                &[resource.clone()],
                location,
                &format!("interface_var_{}", location),
            )?;
        }
    } else if normalize_shader_kind(shader_kind) == ShaderKind::Fragment {
        for resource in &shader_resources.stage_inputs {
            let location =
                ast.get_decoration(resource.id, spirv_cross::spirv::Decoration::Location)?;
            ast.rename_interface_variable(
                &[resource.clone()],
                location,
                &format!("interface_var_{}", location),
            )?;
        }
    }

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

fn normalize_shader_kind(shader_kind: ShaderKind) -> ShaderKind {
    match shader_kind {
        ShaderKind::Vertex | ShaderKind::DefaultVertex => ShaderKind::Vertex,
        ShaderKind::Fragment | ShaderKind::DefaultFragment => ShaderKind::Fragment,
        ShaderKind::Compute | ShaderKind::DefaultCompute => ShaderKind::Compute,
        ShaderKind::Geometry | ShaderKind::DefaultGeometry => ShaderKind::Geometry,
        ShaderKind::TessControl | ShaderKind::DefaultTessControl => ShaderKind::TessControl,
        ShaderKind::TessEvaluation | ShaderKind::DefaultTessEvaluation => {
            ShaderKind::TessEvaluation
        }
        ShaderKind::RayGeneration | ShaderKind::DefaultRayGeneration => ShaderKind::RayGeneration,
        ShaderKind::AnyHit | ShaderKind::DefaultAnyHit => ShaderKind::AnyHit,
        ShaderKind::ClosestHit | ShaderKind::DefaultClosestHit => ShaderKind::ClosestHit,
        ShaderKind::Miss | ShaderKind::DefaultMiss => ShaderKind::Miss,
        ShaderKind::Intersection | ShaderKind::DefaultIntersection => ShaderKind::Intersection,
        ShaderKind::Callable | ShaderKind::DefaultCallable => ShaderKind::Callable,
        ShaderKind::Task | ShaderKind::DefaultTask => ShaderKind::Task,
        ShaderKind::Mesh | ShaderKind::DefaultMesh => ShaderKind::Mesh,
        ShaderKind::InferFromSource => ShaderKind::InferFromSource,
        ShaderKind::SpirvAssembly => ShaderKind::SpirvAssembly,
    }
}
