use rafx_api::{RafxShaderPackage, RafxShaderPackageMetal, RafxShaderPackageVulkan};
use rafx_resources::CookedShaderPackage;
use rafx_resources::{ReflectedEntryPoint, ShaderModuleHash};

pub(crate) fn cook_shader(
    reflected_data: &[ReflectedEntryPoint],
    spv: &[u8],
    metal_source: String,
) -> Result<Vec<u8>, String> {
    let shader_package = RafxShaderPackage {
        vk: Some(RafxShaderPackageVulkan::SpvBytes(spv.to_vec())),

        //TODO: We ideally package binary but this is only possible with apple shader tools installed,
        // which is only available on win/mac. So we'll want a fallback path so that it's not impossible
        // to produce a cooked shader on machines without the tools. (Also the tools don't provide an
        // API so will need to figure out how to compile the shader programmatically.)
        metal: Some(RafxShaderPackageMetal::Src(metal_source)),
    };

    let cooked_shader = CookedShaderPackage {
        entry_points: reflected_data.to_vec(),
        hash: ShaderModuleHash::new(&shader_package),
        shader_package,
    };

    bincode::serialize(&cooked_shader)
        .map_err(|x| format!("Failed to serialize cooked shader: {}", x))
}
