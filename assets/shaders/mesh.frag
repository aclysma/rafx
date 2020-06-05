#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

//
// Per-Frame Pass
//
struct PointLight {
    vec3 position_ws;
    vec3 position_vs;
    vec4 color;
    float range;
    float intensity;
};

struct DirectionalLight {
    vec3 direction_ws;
    vec3 direction_vs;
    vec4 color;
    float intensity;
};

struct SpotLight {
    vec3 position_ws;
    vec3 direction_ws;
    vec3 position_vs;
    vec3 direction_vs;
    vec4 color;
    float spotlight_half_angle;
    float range;
    float intensity;
};

layout (set = 0, binding = 0) uniform PerFrameData {
    vec4 ambient_light;
    uint point_light_count;
    uint directional_light_count;
    uint spot_light_count;
    PointLight point_lights[16];
    DirectionalLight directional_lights[16];
    SpotLight spot_lights[16];
} per_frame_data;

layout (set = 0, binding = 1) uniform sampler smp;

//
// Per-Material Bindings
//
struct MaterialData {
    vec4 base_color_factor;
    vec3 emissive_factor;
    float pad0;
    float metallic_factor;
    float roughness_factor;
    float normal_texture_scale;
    float occlusion_texture_strength;
    float alpha_cutoff;
    bool has_base_color_texture;
    bool has_metallic_roughness_texture;
    bool has_normal_texture;
    bool has_occlusion_texture;
    bool has_emissive_texture;
};

layout (set = 1, binding = 0) uniform MaterialDataUbo {
    MaterialData data;
} material_data_ubo;

layout (set = 1, binding = 1) uniform texture2D base_color_texture;
layout (set = 1, binding = 2) uniform texture2D metallic_roughness_texture;
layout (set = 1, binding = 3) uniform texture2D normal_texture;
layout (set = 1, binding = 4) uniform texture2D occlusion_texture;
layout (set = 1, binding = 5) uniform texture2D emissive_texture;

layout (location = 0) in vec3 in_position_vs;
layout (location = 1) in vec3 in_normal_vs;
// w component is a sign value (-1 or +1) indicating handedness of the tangent basis
// see GLTF spec for more info
layout (location = 2) in vec3 in_tangent_vs;
layout (location = 3) in vec3 in_binormal_vs;
layout (location = 4) in vec2 in_uv;

// Force early depth testing, this is likely not strictly necessary
layout(early_fragment_tests) in;

layout (location = 0) out vec4 out_color;

vec4 normal_map(
    mat3 tangent_normal_binormal, 
    texture2D t, 
    sampler s, 
    vec2 uv
) {
    // Sample the normal and unflatten it from the texture (i.e. convert
    // range of [0, 1] to [-1, 1])
    vec3 normal = texture(sampler2D(t, s), uv).xyz;
    normal = normal * 2.0 - 1.0;

    // Transform the normal from the texture with the TNB matrix, which will put
    // it into the TNB's space (view space))
    normal = normal * tangent_normal_binormal;
    return normalize(vec4(normal, 0.0));
}

vec4 diffuse_light(
    vec3 surface_to_light_dir, 
    vec3 normal, 
    vec4 light_color
) {
    // Diffuse light - just dot the normal vector with the surface to light dir.
    float NdotL = max(dot(surface_to_light_dir, normal), 0);
    return light_color * NdotL;
}

vec4 specular_light_phong(
    vec3 surface_to_light_dir, 
    vec3 surface_to_eye_dir, 
    vec3 normal, 
    vec4 light_color
) {
    // Calculate the angle that light might reflect at
    vec3 reflect_dir = normalize(reflect(-surface_to_light_dir, normal));

    // Dot the reflection with the view angle
    float RdotV = max(dot(reflect_dir, surface_to_eye_dir), 0);

    // Raise to a power to get the specular effect on a narrow viewing angle
    return light_color * pow(RdotV, 4.0); // hardcode a spec power, will switch to BSDF later
}

vec4 specular_light_blinn_phong(
    vec3 surface_to_light_dir,
    vec3 surface_to_eye_dir,
    vec3 normal,
    vec4 light_color
) {
    // Calculate the angle that light might reflect at
    vec3 halfway_dir = normalize(surface_to_light_dir + surface_to_eye_dir);

    // Dot the reflection with the view angle
    float RdotV = max(dot(normal, halfway_dir), 0);

    // Raise to a power to get the specular effect on a narrow viewing angle
    return light_color * pow(RdotV, 4.0); // hardcode a spec power, will switch to BSDF later
}

vec4 specular_light(
    vec3 surface_to_light_dir,
    vec3 surface_to_eye_dir,
    vec3 normal,
    vec4 light_color
) {
    return specular_light_blinn_phong(
        surface_to_light_dir,
        surface_to_eye_dir,
        normal,
        light_color
    );
}

float attenuate_light(
    float light_range, 
    float distance
) {
    // Full lighting until 75% away, then step down to no lighting
    return 1.0 - smoothstep(light_range * .75, light_range, distance);
}

vec4 point_light(
    PointLight light,
    MaterialData material,
    vec3 surface_to_eye_dir_vs,
    vec3 surface_position_vs,
    vec3 normal_vs
) {
    // Get the distance to the light and normalize the surface_to_light direction. (Not
    // using normalize since we want the distance too)
    vec3 surface_to_light_dir = light.position_vs - surface_position_vs;
    float distance = length(surface_to_light_dir);
    surface_to_light_dir = surface_to_light_dir / distance;

    // Figure out the falloff of light intensity due to distance from light source
    float attenuation = attenuate_light(light.range, distance);

    // Calculate lighting components
    vec4 diffuse = diffuse_light(surface_to_light_dir, normal_vs, light.color) * attenuation * light.intensity;
    vec4 specular = specular_light(surface_to_light_dir, surface_to_eye_dir_vs, normal_vs, light.color) * attenuation * light.intensity;
    return diffuse + specular;
}

float spotlight_cone_falloff(
    vec3 surface_to_light_dir,
    vec3 spotlight_dir,
    float spotlight_half_angle
) {
    // If we dot -spotlight_dir with surface_to_light_dir:
    // - the result will be 1 if the spotlight is pointed straight at the surface position
    // - the result will be 0 if the spotlight direction is orthogonal to the surface position
    float cos_angle = dot(-spotlight_dir, surface_to_light_dir);

    // spotlight_half_angle will indicate the minimum result necessary to receive lighting contribution
    // this indicates the "edge" of the ring on the surface formed by the spotlight where there is no longer a lighting
    // contribution
    float min_cos = cos(spotlight_half_angle);

    // Pick an angle at which to start reducing lighting contribution based on direction of the spotlight
    float max_cos = mix(min_cos, 1, 0.5); // mix is lerp

    // based on the angle found in cos_angle, calculate the contribution
    return smoothstep(min_cos, max_cos, cos_angle);
}

vec4 spot_light(
    SpotLight light,
    MaterialData material,
    vec3 surface_to_eye_dir_vs,
    vec3 surface_position_vs,
    vec3 normal_vs
) {
    // Get the distance to the light and normalize the surface_to_light direction. (Not
    // using normalize since we want the distance too)
    vec3 surface_to_light_dir = light.position_vs - surface_position_vs;
    float distance = length(surface_to_light_dir);
    surface_to_light_dir = surface_to_light_dir / distance;

    // Figure out the falloff of light intensity due to distance from light source
    float attenuation = attenuate_light(light.range, distance);
    float spotlight_direction_intensity = spotlight_cone_falloff(
        surface_to_light_dir,
        light.direction_vs,
        light.spotlight_half_angle
    );

    // Calculate lighting components
    vec4 diffuse = diffuse_light(surface_to_light_dir, normal_vs, light.color) * attenuation * light.intensity * spotlight_direction_intensity;
    vec4 specular = specular_light(surface_to_light_dir, surface_to_eye_dir_vs, normal_vs, light.color) * attenuation * light.intensity * spotlight_direction_intensity;
    return diffuse + specular;
}

vec4 directional_light(
    DirectionalLight light,
    MaterialData material,
    vec3 surface_to_eye_dir_vs,
    vec3 surface_position_vs,
    vec3 normal_vs
) {
    vec3 surface_to_light_dir = -light.direction_vs;

    vec4 diffuse = diffuse_light(surface_to_light_dir, normal_vs, light.color) * light.intensity;
    vec4 specular = specular_light(surface_to_light_dir, surface_to_eye_dir_vs, normal_vs, light.color) * light.intensity;
    return diffuse + specular;
}

// References:
// https://www.3dgep.com/forward-plus/
// - Basic framework for forward/deferred/forward+ in non-PBR
// https://learnopengl.com/PBR/Theory
// - PBR
// https://cdn2.unrealengine.com/Resources/files/2013SiggraphPresentationsNotes-26915738.pdf

const float PI = 3.14159265359;

//
// Normal distribution function approximates the relative surface area where microfacets are aligned to the halfway
// vector, producing specular-like results. (GGX/Trowbridge-Reitz)
//
float ndf_ggx(
    vec3 n,
    vec3 h,
    float roughness
) {
    // disney/epic squre roughness
    // https://cdn2.unrealengine.com/Resources/files/2013SiggraphPresentationsNotes-26915738.pdf
    float a = roughness * roughness;
    float a2 = a * a;

    float n_dot_h = max(dot(n, h), 0.0);
    float bottom_part = (n_dot_h * n_dot_h * (a2 - 1.0) + 1.0);
    float bottom = PI * bottom_part * bottom_part;
    return a2 / bottom;
}

//
// geometric attenuation, approximates light rays being trapped within micro-surfaces of the material
//
float geometric_attenuation_schlick_ggx(
    float dot_product,
    float k
) {
    float bottom = (dot_product * (1.0 - k)) + k;
    return dot_product / bottom;
}

float geometric_attenuation_smith(
    vec3 n,
    vec3 v,
    vec3 l,
    float roughness
) {
    // This is appropriate for analytic lights, not image-based
    float r_plus_1 = (roughness + 1.0);
    float k = r_plus_1 * r_plus_1 / 8.0;

    float v_factor = geometric_attenuation_schlick_ggx(max(dot(n, v), 0.0), k);
    float l_factor = geometric_attenuation_schlick_ggx(max(dot(n, l), 0.0), k);
    return v_factor * l_factor;
}

//
// fresnel_base is specular reflectance at normal incidence
//
vec3 fresnel_schlick(
    vec3 v,
    vec3 h,
    vec3 fresnel_base
) {
    float v_dot_h = max(dot(v, h), 0.0);

    // approximation for pow(1 - v_dot_h, 5)
    // https://seblagarde.wordpress.com/2012/06/03/spherical-gaussien-approximation-for-blinn-phong-phong-and-fresnel/
    // https://cdn2.unrealengine.com/Resources/files/2013SiggraphPresentationsNotes-26915738.pdf
    return fresnel_base + (1.0 - fresnel_base) * exp2((-5.55473 * v_dot_h - 6.98316) * v_dot_h);
}

vec3 point_light_pbr(
    PointLight light,
    MaterialData material,
    vec3 surface_to_eye_dir_vs,
    vec3 surface_position_vs,
    vec3 normal_vs,
    vec3 F0,
    vec3 base_color,
    float roughness,
    float metalness
) {
    // Get the distance to the light and normalize the surface_to_light direction. (Not
    // using normalize since we want the distance too)
    vec3 surface_to_light_dir_vs = light.position_vs - surface_position_vs;
    float distance = length(surface_to_light_dir_vs);
    surface_to_light_dir_vs = surface_to_light_dir_vs / distance;

    vec3 halfway_dir_vs = normalize(surface_to_light_dir_vs + surface_to_eye_dir_vs);

    // Figure out the falloff of light intensity due to distance from light source
    float attenuation = 1.0 / (distance * distance);
    vec3 radiance = light.color.rgb * attenuation * light.intensity;

    float NDF = ndf_ggx(normal_vs, halfway_dir_vs, roughness);
    float G = geometric_attenuation_smith(normal_vs, surface_to_eye_dir_vs, surface_to_light_dir_vs, roughness);
    vec3 F = fresnel_schlick(surface_to_eye_dir_vs, halfway_dir_vs, F0);

    // fresnel defines ratio of light energy that contributes to specular lighting
    vec3 fresnel_specular = F;
    vec3 fresnel_diffuse = vec3(1.0) - fresnel_specular;

    // As the surface becomes more metallic, remove the diffuse term
    fresnel_diffuse *= 1.0 - metalness;

    // Cook-Torrance specular BRDF
    float n_dot_l = max(dot(normal_vs, surface_to_light_dir_vs), 0.0);
    float n_dot_v = max(dot(normal_vs, surface_to_eye_dir_vs), 0.0);
    vec3 top = NDF * G * F;
    float bottom = 4.0 * n_dot_v * n_dot_l;
    vec3 specular = top / max(bottom, 0.001);

    return ((fresnel_diffuse * base_color / PI) + specular) * radiance * n_dot_l;
}






void main() {
    // Sample the base color, if it exists
    vec4 base_color = material_data_ubo.data.base_color_factor;
    if (material_data_ubo.data.has_base_color_texture) {
        base_color *= texture(sampler2D(base_color_texture, smp), in_uv);
    }

    // Sample the emissive color, if it exists
    vec4 emissive_color = vec4(material_data_ubo.data.emissive_factor, 1);
    if (material_data_ubo.data.has_emissive_texture) {
        emissive_color *= texture(sampler2D(emissive_texture, smp), in_uv);
        base_color = vec4(1.0, 1.0, 0.0, 1.0);
    }

    // Sample metalness/roughness
    float metalness = material_data_ubo.data.metallic_factor;
    float roughness = material_data_ubo.data.roughness_factor;
    if (material_data_ubo.data.has_metallic_roughness_texture) {
        vec4 sampled = texture(sampler2D(metallic_roughness_texture, smp), in_uv);
        metalness *= sampled.r;
        roughness *= sampled.g;
    }

    // Calculate the normal (use the normal map if it exists)
    vec4 normal_vs;
    if (material_data_ubo.data.has_normal_texture) {
        mat3 tbn = mat3(in_tangent_vs, in_binormal_vs, in_normal_vs);
        normal_vs = normal_map(tbn, normal_texture, smp, in_uv);
    } else {
        normal_vs = normalize(vec4(in_normal_vs, 0));
    }

    //TOOD: AO

    vec3 eye_position_vs = vec3(0, 0, 0);
    vec3 surface_to_eye_vs = normalize(eye_position_vs - in_position_vs);

    // used in fresnel, non-metals use 0.04 and metals use the base color
    vec3 fresnel_base = vec3(0.04);
    fresnel_base = mix(fresnel_base, base_color.rgb, vec3(metalness));
    
    // Point Lights
    vec3 total_light = vec3(0.0);
    for (uint i = 0; i < per_frame_data.point_light_count; ++i) {
        // TODO: Early out by distance?

//        total_light += point_light(
//            per_frame_data.point_lights[i],
//            material_data_ubo.data,
//            surface_to_eye_vs,
//            in_position_vs,
//            in_normal_vs
//        );

        total_light += point_light_pbr(
            per_frame_data.point_lights[i],
            material_data_ubo.data,
            surface_to_eye_vs,
            in_position_vs,
            in_normal_vs,
            fresnel_base,
            base_color.rgb,
            roughness,
            metalness
        );
    }

//    // Spot Lights
//    for (uint i = 0; i < per_frame_data.spot_light_count; ++i) {
//        // TODO: Early out by distance?
//
//        total_light += spot_light(
//            per_frame_data.spot_lights[i],
//            material_data_ubo.data,
//            surface_to_eye_vs,
//            in_position_vs,
//            in_normal_vs
//        ).rgb;
//    }
//
//    // directional Lights
//    for (uint i = 0; i < per_frame_data.directional_light_count; ++i) {
//        total_light += directional_light(
//            per_frame_data.directional_lights[i],
//            material_data_ubo.data,
//            surface_to_eye_vs,
//            in_position_vs,
//            in_normal_vs
//        ).rgb;
//    }

//    base_color *= (per_frame_data.ambient_light + total_light);
//    out_color = emissive_color + base_color;

    vec3 ambient = per_frame_data.ambient_light.rgb * base_color.rgb; //TODO: Multiply ao in here
    vec3 color = ambient + total_light + emissive_color.rgb;


    // tonemapping
    vec3 mapped = color.rgb / (color.rgb + vec3(1.0));

    // gamma correction
    const float gamma = 2.2;
    //mapped = pow(mapped, vec3(1.0 / gamma));

    // output
    out_color = vec4(mapped, base_color.a);



//    out_color = color;


//
//    //TODO: Not sure alpha should be involved here...
//    //color = color / (color + vec4(1.0));
//    //color = vec4(pow(color.xyz, vec3(1.0/2.2)), color.w);
//
//    out_color = color;

//
//    //base_color *= (per_frame_data.ambient_light + lighting);
//    //out_color = emissive_color + base_color;
}
