use rafx_api::{
    RafxShaderPackage, RafxShaderPackageGles2, RafxShaderPackageGles3, RafxShaderPackageMetal,
    RafxShaderPackageVulkan,
};
use rafx_framework::CookedShaderPackage;
use rafx_framework::{ReflectedEntryPoint, ShaderModuleHash};

pub(crate) fn cook_shader(
    reflected_data: &[ReflectedEntryPoint],
    vk_spv: Option<&Vec<u8>>,
    metal_source: Option<String>,
    gles2_source: Option<String>,
    gles3_source: Option<String>,
) -> Result<Vec<u8>, String> {
    let shader_package = RafxShaderPackage {
        vk: vk_spv.map(|x| RafxShaderPackageVulkan::SpvBytes(x.to_vec())),

        //TODO: We ideally package binary but this is only possible with apple shader tools installed,
        // which is only available on win/mac. So we'll want a fallback path so that it's not impossible
        // to produce a cooked shader on machines without the tools. (Also the tools don't provide an
        // API so will need to figure out how to compile the shader programmatically.)
        metal: metal_source.map(|x| RafxShaderPackageMetal::Src(x)),

        gles2: gles2_source.map(|x| RafxShaderPackageGles2::Src(x)),
        gles3: gles3_source.map(|x| RafxShaderPackageGles3::Src(x)),
    };

    let cooked_shader = CookedShaderPackage {
        entry_points: reflected_data.to_vec(),
        hash: ShaderModuleHash::new(&shader_package),
        shader_package,
    };

    bincode::serialize(&cooked_shader)
        .map_err(|x| format!("Failed to serialize cooked shader: {}", x))
}
