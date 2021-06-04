use std::error::Error;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

mod parse_source;
use parse_source::AnnotationText;
use parse_source::DeclarationText;

mod parse_declarations;

mod include;
use crate::reflect::ShaderProcessorRefectionData;
use fnv::FnvHashSet;
use include::include_impl;
use include::IncludeType;
use shaderc::ShaderKind;
use spirv_cross::glsl::Target;
use spirv_cross::spirv::{Ast, ShaderResources};

mod codegen;

mod cook;

mod reflect;

mod shader_types;

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
    pub glsl_files: Option<Vec<PathBuf>>,
    #[structopt(name = "spv-path", long, parse(from_os_str))]
    pub spv_path: Option<PathBuf>,
    #[structopt(name = "rs-path", long, parse(from_os_str))]
    pub rs_path: Option<PathBuf>,
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
    #[structopt(name = "package-metal", long)]
    pub package_metal: bool,
    #[structopt(name = "package-gles2", long)]
    pub package_gles2: bool,
    #[structopt(name = "package-gles3", long)]
    pub package_gles3: bool,
    #[structopt(name = "package-all", long)]
    pub package_all: bool,
}

pub fn run(args: &ShaderProcessorArgs) -> Result<(), Box<dyn Error>> {
    log::trace!("Shader processor args: {:#?}", args);

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
            args.rs_file.as_ref(),
            args.metal_generated_src_file.as_ref(),
            args.gles2_generated_src_file.as_ref(),
            args.gles3_generated_src_file.as_ref(),
            args.cooked_shader_file.as_ref(),
            shader_kind,
            &args,
        )
        .map_err(|x| format!("{}: {}", glsl_file.to_string_lossy(), x.to_string()))?;

        Ok(())
    } else if let Some(glsl_file_patterns) = &args.glsl_files {
        //
        // Handle a batch of file patterns (such as *.frag) via --glsl_files. Infer output files
        // based on other args given in the form of output directories
        //

        // This will accumulate rust module names so we can produce a lib.rs if needed
        let mut module_names = Vec::default();

        for glsl_file in glsl_file_patterns {
            log::trace!("input file pattern: {:?}", glsl_file);
            for glob in glob::glob(glsl_file.to_str().unwrap())? {
                //
                // Determine the files we will write out
                //
                let glsl_file = glob?;
                log::info!("Processing file {:?}", glsl_file);

                let file_name = glsl_file
                    .file_name()
                    .ok_or_else(|| "Failed to get the filename from glob match".to_string())?
                    .to_string_lossy();

                let spv_name = format!("{}.spv", file_name);
                let spv_path = args.spv_path.as_ref().map(|x| x.join(spv_name));

                let rs_module_name = file_name.to_string().to_lowercase().replace(".", "_");
                let rs_name = format!("{}.rs", rs_module_name);
                let rs_path = args.rs_path.as_ref().map(|x| x.join(rs_name));

                let metal_src_name = format!("{}.metal", file_name);
                let metal_generated_src_path = args
                    .metal_generated_src_path
                    .as_ref()
                    .map(|x| x.join(metal_src_name));

                let gles2_src_name = format!("{}.gles2", file_name);
                let gles2_generated_src_path = args
                    .gles2_generated_src_path
                    .as_ref()
                    .map(|x| x.join(gles2_src_name));

                let gles3_src_name = format!("{}.gles3", file_name);
                let gles3_generated_src_path = args
                    .gles3_generated_src_path
                    .as_ref()
                    .map(|x| x.join(gles3_src_name));

                let cooked_shader_name = format!("{}.cookedshaderpackage", file_name);
                let cooked_shader_path = args
                    .cooked_shaders_path
                    .as_ref()
                    .map(|x| x.join(cooked_shader_name));

                //
                // Try to determine what kind of shader this is from the file name
                //
                let shader_kind = shader_kind_from_args(args)
                    .or_else(|| deduce_default_shader_kind_from_path(&glsl_file))
                    .unwrap_or(shaderc::ShaderKind::InferFromSource);

                //
                // Process this shader and write to output files
                //
                process_glsl_shader(
                    &glsl_file,
                    spv_path.as_ref(),
                    rs_path.as_ref(),
                    metal_generated_src_path.as_ref(),
                    gles2_generated_src_path.as_ref(),
                    gles3_generated_src_path.as_ref(),
                    cooked_shader_path.as_ref(),
                    shader_kind,
                    &args,
                )
                .map_err(|x| format!("{}: {}", glsl_file.to_string_lossy(), x.to_string()))?;

                //
                // Add the module name to this list so we can generate a lib.rs later
                //
                if rs_path.is_some() {
                    module_names.push(rs_module_name.clone());
                }
            }
        }

        //
        // Generate a lib.rs that includes all the compiled shaders
        //
        if let Some(rs_path) = &args.rs_path {
            let mut lib_file_string = String::default();
            lib_file_string += "// This code is auto-generated by the shader processor.\n\n";

            for module_name in module_names {
                lib_file_string += &format!("pub mod {};\n", module_name);
            }

            let lib_file_path = rs_path.join("lib.rs");
            log::trace!("Write lib file {:?}", lib_file_path);
            std::fs::write(lib_file_path, lib_file_string)?;
        }

        Ok(())
    } else {
        Ok(())
    }
}

fn process_glsl_shader(
    glsl_file: &Path,
    spv_file: Option<&PathBuf>,
    rs_file: Option<&PathBuf>,
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
    log::trace!("metal: {:?}", metal_generated_src_file);
    log::trace!("gles2: {:?}", gles2_generated_src_file);
    log::trace!("gles3: {:?}", gles3_generated_src_file);
    log::trace!("cooked: {:?}", cooked_shader_file);
    log::trace!("shader kind: {:?}", shader_kind);

    let package_vk = (args.package_all || args.package_vk) && cooked_shader_file.is_some();
    let package_metal = (args.package_all || args.package_metal) && cooked_shader_file.is_some();
    let package_gles2 = (args.package_all || args.package_gles2) && cooked_shader_file.is_some();
    let package_gles3 = (args.package_all || args.package_gles3) && cooked_shader_file.is_some();

    log::trace!(
        "package VK: {} Metal: {} GLES2: {} GLES3: {}",
        package_vk,
        package_metal,
        package_gles2,
        package_gles3
    );

    if cooked_shader_file.is_some()
        && !(package_vk || package_metal || package_gles2 || package_gles3)
    {
        Err("A cooked shader file or path was specified but no shader types are specified to package. Pass --package-vk, --package-metal, --package-gles2, --package-gles3, or --package-all")?;
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
    let mut compiler = shaderc::Compiler::new().unwrap();

    log::trace!("{:?}: compile unoptimized", glsl_file);
    let unoptimized_compile_spirv_result = {
        let mut compile_options = shaderc::CompileOptions::new().unwrap();
        compile_options.set_include_callback(include::shaderc_include_callback);

        compiler.compile_into_spirv(
            &code,
            shader_kind,
            glsl_file.to_str().unwrap(),
            entry_point_name,
            Some(&compile_options),
        )?
    };

    //
    // Read the unoptimized spv into spirv_cross so that we can grab reflection data
    //
    log::trace!("{:?}: read spirv_cross module", glsl_file);
    let spirv_cross_module =
        spirv_cross::spirv::Module::from_words(unoptimized_compile_spirv_result.as_binary());

    //TEMP: Create this for now, planning to remove the dependency later
    log::trace!("{:?}: read spirv_reflect module", glsl_file);
    let spirv_reflect_module =
        spirv_reflect::create_shader_module(unoptimized_compile_spirv_result.as_binary_u8())?;

    //
    // Parse the shader code to find all declared resources. This is a high-level parse of the file
    // to extract the bits we care about along with the comments that are associated with those bits
    //
    log::trace!("{:?}: parse glsl", glsl_file);
    let parsed_source = parse_source::parse_glsl(&glsl_file)?;

    //
    // Parse the declarations that were extracted from the source file
    //
    log::trace!("{:?}: parse declarations", glsl_file);
    let parsed_declarations = parse_declarations::parse_declarations(&parsed_source.declarations)?;

    // example usage of spirv_cross. We can provide options here to modify the shader
    // programmatically. This could use annotations to drive this
    log::trace!("{:?}: generate spirv_cross ast", glsl_file);
    let mut spirv_cross_glsl_options = spirv_cross::glsl::CompilerOptions::default();
    spirv_cross_glsl_options.vulkan_semantics = true;
    let mut ast = spirv_cross::spirv::Ast::<spirv_cross::glsl::Target>::parse(&spirv_cross_module)?;
    ast.set_compiler_options(&spirv_cross_glsl_options)?;

    log::trace!("{:?}: generate shader types", glsl_file);
    let mut user_types = shader_types::create_user_type_lookup(&parsed_declarations)?;
    let builtin_types = shader_types::create_builtin_type_lookup();

    let mut reflected_data = if rs_file.is_some()
        || cooked_shader_file.is_some()
        || metal_generated_src_file.is_some()
        || gles2_generated_src_file.is_some()
    {
        log::trace!("{:?}: generate reflection data", glsl_file);
        let require_semantics = cooked_shader_file.is_some();
        Some(reflect::reflect_data(
            &builtin_types,
            &user_types,
            &ast,
            &parsed_declarations,
            require_semantics,
        )?)
    } else {
        None
    };

    let rust_code = if rs_file.is_some() {
        log::trace!("{:?}: generate rust code", glsl_file);
        let reflected_entry_point = reflected_data
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
            &builtin_types,
            &mut user_types,
            &parsed_declarations,
            &spirv_reflect_module,
            &reflected_entry_point,
        )?)
    } else {
        None
    };

    //TODO: spirv_reflect does not include sampler/textur ein some cases
    //TODO: spirv_cross is generating a spurious combined image/sampler
    //TODO: How to generate data in cook_shader

    //
    // If needed, compile the shader in release mode, otherwise just keep our unoptimized spv from
    // before. This is the spv that we will write out.
    //
    //TODO: Should we compile what comes out of spirv cross?
    //ast.build_combined_image_samplers();
    //let compiled = ast.compile()?;
    let output_spv = if args.optimize_shaders {
        log::trace!("{:?}: compile optimized", glsl_file);
        let mut compile_options = shaderc::CompileOptions::new().unwrap();
        compile_options.set_include_callback(include::shaderc_include_callback);
        compile_options.set_optimization_level(shaderc::OptimizationLevel::Performance);
        //NOTE: Could also use shaderc::OptimizationLevel::Size

        compiler
            .compile_into_spirv(
                &code,
                shader_kind,
                glsl_file.to_str().unwrap(),
                entry_point_name,
                Some(&compile_options),
            )?
            .as_binary_u8()
            .to_vec()
    } else {
        log::trace!("{:?}: do not recompile optimized", glsl_file);
        unoptimized_compile_spirv_result.as_binary_u8().to_vec()
    };

    let metal_src = if metal_generated_src_file.is_some() || package_metal {
        log::trace!("{:?}: create msl", glsl_file);
        let mut msl_ast =
            spirv_cross::spirv::Ast::<spirv_cross::msl::Target>::parse(&spirv_cross_module)?;
        let mut spirv_cross_msl_options = spirv_cross::msl::CompilerOptions::default();
        spirv_cross_msl_options.version = spirv_cross::msl::Version::V2_0;
        spirv_cross_msl_options.enable_argument_buffers = true;
        spirv_cross_msl_options.force_active_argument_buffer_resources = true;

        //TODO: Set this up
        spirv_cross_msl_options.resource_binding_overrides = reflected_data
            .as_ref()
            .unwrap()
            .msl_argument_buffer_assignments
            .clone();
        //spirv_cross_msl_options.vertex_attribute_overrides
        spirv_cross_msl_options.const_samplers =
            reflected_data.as_ref().unwrap().msl_const_samplers.clone();

        msl_ast.set_compiler_options(&spirv_cross_msl_options)?;
        let metal_src = msl_ast.compile()?;

        Some(metal_src)
    } else {
        None
    };

    let gles2_src = if gles2_generated_src_file.is_some() || package_gles2 {
        log::trace!("{:?}: create gles2", glsl_file);
        let mut gles2_ast =
            spirv_cross::spirv::Ast::<spirv_cross::glsl::Target>::parse(&spirv_cross_module)?;
        let mut spirv_cross_gles2_options = spirv_cross::glsl::CompilerOptions::default();
        spirv_cross_gles2_options.version = spirv_cross::glsl::Version::V1_00Es;
        spirv_cross_gles2_options.vulkan_semantics = false;
        spirv_cross_gles2_options.vertex.transform_clip_space = true;
        spirv_cross_gles2_options.vertex.invert_y = true;

        let shader_resources = ast.get_shader_resources()?;

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

        rename_gl_samplers(&mut reflected_data, &mut gles2_ast)?;
        rename_gl_in_out_attributes(shader_kind, &mut gles2_ast, &shader_resources)?;

        gles2_ast.set_compiler_options(&spirv_cross_gles2_options)?;
        let gles2_src = gles2_ast.compile()?;

        Some(gles2_src)
    } else {
        None
    };

    let gles3_src = if gles3_generated_src_file.is_some() || package_gles3 {
        log::trace!("{:?}: create gles3", glsl_file);
        let mut gles3_ast =
            spirv_cross::spirv::Ast::<spirv_cross::glsl::Target>::parse(&spirv_cross_module)?;
        let mut spirv_cross_gles3_options = spirv_cross::glsl::CompilerOptions::default();
        spirv_cross_gles3_options.version = spirv_cross::glsl::Version::V3_00Es;
        spirv_cross_gles3_options.vulkan_semantics = false;
        spirv_cross_gles3_options.vertex.transform_clip_space = true;
        spirv_cross_gles3_options.vertex.invert_y = true;

        let shader_resources = ast.get_shader_resources()?;

        rename_gl_samplers(&mut reflected_data, &mut gles3_ast)?;
        rename_gl_in_out_attributes(shader_kind, &mut gles3_ast, &shader_resources)?;

        gles3_ast.set_compiler_options(&spirv_cross_gles3_options)?;
        let gles3_src = gles3_ast.compile()?;

        Some(gles3_src)
    } else {
        None
    };

    // Don't worry about the return value
    log::trace!("{:?}: cook shader", glsl_file);
    let cooked_shader = if cooked_shader_file.is_some() {
        let output_spv = if package_vk { Some(&output_spv) } else { None };

        let metal_src = if package_metal {
            Some(metal_src.as_ref().unwrap().clone())
        } else {
            None
        };

        let gles2_src = if package_gles2 {
            Some(gles2_src.as_ref().unwrap().clone())
        } else {
            None
        };

        let gles3_src = if package_gles3 {
            Some(gles3_src.as_ref().unwrap().clone())
        } else {
            None
        };

        Some(cook::cook_shader(
            &reflected_data.as_ref().unwrap().reflection,
            output_spv,
            metal_src,
            gles2_src,
            gles3_src,
        )?)
    } else {
        None
    };

    //
    // Write out the spv and rust files if desired
    //
    if let Some(spv_file) = &spv_file {
        write_output_file(spv_file, output_spv)?;
    }

    if let Some(rs_file) = &rs_file {
        write_output_file(rs_file, rust_code.unwrap())?;
    }

    if let Some(metal_generated_src_file) = &metal_generated_src_file {
        write_output_file(metal_generated_src_file, metal_src.unwrap())?;
    }

    if let Some(gles2_generated_src_file) = &gles2_generated_src_file {
        write_output_file(gles2_generated_src_file, gles2_src.unwrap())?;
    }

    if let Some(gles3_generated_src_file) = &gles3_generated_src_file {
        write_output_file(gles3_generated_src_file, gles3_src.unwrap())?;
    }

    if let Some(cooked_shader_file) = &cooked_shader_file {
        write_output_file(cooked_shader_file, cooked_shader.unwrap())?;
    }

    Ok(())
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
