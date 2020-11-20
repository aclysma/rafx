use renderer_assets::assets::reflect::ReflectedEntryPoint;
use renderer_assets::CookedShader;

pub(crate) fn cook_shader(
    reflected_data: &[ReflectedEntryPoint],
    spv: &[u8],
) -> Result<Vec<u8>, String> {
    let cooked_shader = CookedShader {
        entry_points: reflected_data.to_vec(),
        spv: spv.to_vec(),
    };

    bincode::serialize(&cooked_shader)
        .map_err(|x| format!("Failed to serialize cooked shader: {}", x))
}
