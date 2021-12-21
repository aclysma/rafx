// @[export]
// @[slot_name("base_color_texture")]
layout (set = 1, binding = 1) uniform texture2D base_color_texture;

// @[export]
// @[slot_name("metallic_roughness_texture")]
layout (set = 1, binding = 2) uniform texture2D metallic_roughness_texture;

// @[export]
// @[slot_name("normal_texture")]
layout (set = 1, binding = 3) uniform texture2D normal_texture;

// @[export]
// @[slot_name("occlusion_texture")]
layout (set = 1, binding = 4) uniform texture2D occlusion_texture;

// @[export]
// @[slot_name("emissive_texture")]
layout (set = 1, binding = 5) uniform texture2D emissive_texture;