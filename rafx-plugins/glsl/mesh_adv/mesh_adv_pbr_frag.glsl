layout (location = 0) in vec3 in_position_vs;
layout (location = 1) in vec3 in_normal_vs;
// w component is a sign value (-1 or +1) indicating handedness of the tangent basis
// see GLTF spec for more info
layout (location = 2) in vec3 in_tangent_vs;
layout (location = 3) in vec3 in_binormal_vs;
layout (location = 4) in vec2 in_uv;
layout (location = 5) in vec4 in_position_ws;
layout (location = 6) in mat3 in_model_view;
//layout (location = 7) in mat3 in_model_view;
//layout (location = 8) in mat3 in_model_view;
layout (location = 9) flat in uint in_instance_index;

// References:
// https://www.3dgep.com/forward-plus/
// - Basic framework for forward/deferred/forward+ in non-PBR
// https://learnopengl.com/PBR/Theory
// - PBR
// https://cdn2.unrealengine.com/Resources/files/2013SiggraphPresentationsNotes-26915738.pdf
// https://google.github.io/filament/Filament.md.html
//
// The unreal paper is straightforward and practical for PBR. The filament one is more thorough and covers some slower
// but more accurate approaches.

const float PI = 3.14159265359;

//TODO: Shader processor can't handle consts
//const int MAX_POINT_LIGHTS = 16;
//const int MAX_DIRECTIONAL_LIGHTS = 16;
//const int MAX_SPOT_LIGHTS = 16;
//const int MAX_SHADOWS = MAX_DIRECTIONAL_LIGHTS * MAX_POINT_LIGHTS * MAX_SPOT_LIGHTS;

// These are determined by trial and error. A deeper Z projection requires lower numbers here so this may need to be
// per light based on Z-depth
//
// Also, currently the depth values from ortho projections are compressed near 1.0 and perspective near 0.0. Don't know
// why this is happening and it may be why these need such different values
//
// Make sure to tune these with 1-sample PCF rather than multi-sample PCF as multi-sample will hide shadow acne
//
// If the numbers are high, you'll get peter-panning
// If the nubers are low, you'll get noise (shadow acne)
//
// These were tuned with near/far distances of 0.1 to 100.0 reversed Z
//
const float SPOT_LIGHT_SHADOW_MAP_BIAS_MULTIPLIER = 1.0;
const float DIRECTIONAL_LIGHT_SHADOW_MAP_BIAS_MULTIPLIER = 1.0;
//const float POINT_LIGHT_SHADOW_MAP_BIAS_MULTIPLIER = 0.01;
// Cube maps have their own codepath so no constant here yet

// The max is used when light is hitting at an angle (near orthogonal to normal). Min is used when light is hitting
// directly
//
// Tuning steps:
// - Use a single directional light (DIRECTIONAL_LIGHT_SHADOW_MAP_BIAS_MULTIPLIER = 1.0)
//   - Test a light almost vertical with the ground (almost parallel to normal). Raise SHADOW_MAP_BIAS_MIN until
//     shadow acne is gone
//   - Test a light almost horizontal with the ground (almost orthogonal to normal). Raise SHADOW_MAP_BIAS_MAX until
//     shadow acne is gone
//     - There may be a repeating pattern on the ground, the frequency this repeats at in distance is how much peter-panning
//       there will be once the min is high enough to get rid of the noise
//   - Test angles at in-between angles. You can play with the bias function to square or cube the bias_angle_factor
// - Test other lights and adjust their bias
// - NOTE: Factors at play here are (probably) Z depth of projection and resolution of shadow texture
//
// TODO: Investigate if per-light bias multiplier (i.e. SPOT_LIGHT_SHADOW_MAP_BIAS_MULTIPLIER) should apply to SHADOW_MAP_BIAS_MIN
const float SHADOW_MAP_BIAS_MAX = 0.01;
const float SHADOW_MAP_BIAS_MIN = 0.0005;

//#define PCF_DISABLED
//#define PCF_SAMPLE_1
#define PCF_SAMPLE_9
//#define PCF_SAMPLE_25

#define PCF_CUBE_SAMPLE_1
//#define PCF_CUBE_SAMPLE_8
//#define PCF_CUBE_SAMPLE_20
//#define PCF_CUBE_SAMPLE_64

//#define DEBUG_RENDER_PERCENT_LIT

#ifdef PBR_TEXTURES
// Passing texture/sampler through like this breaks reflection metadata so for now just grab global data
vec4 normal_map(
    int normal_texture,
    mat3 tangent_binormal_normal,
    vec2 uv
) {
    // Sample the normal and unflatten it from the texture (i.e. convert
    // range of [0, 1] to [-1, 1])
    vec3 normal = texture(sampler2D(all_material_textures[normal_texture], smp), uv, per_view_data.mip_bias).xyz;
    normal = normal * 2.0 - 1.0;
    normal.z = 0.0;
    normal.z = sqrt(1.0 - dot(normal, normal));
    normal.x = -normal.x;
    normal.y = -normal.y;

    // Transform the normal from the texture with the TNB matrix, which will put
    // it into the TNB's space (view space))
    normal = tangent_binormal_normal * normal;
    return normalize(vec4(normal, 0.0));
}
#endif

//TODO: Set up dummy texture so all bindings can be populated
//TODO: Fix bias adjustment in spotlights?

// Determine the depth value that would be returned from a cubemap if the depth sample was of the given surface
// Since cubemaps have defined projections, we just need the near plane and far plane.
float calculate_cubemap_equivalent_depth(vec3 light_to_surface_ws, float near, float far)
{
    // Find the absolute value of the largest component of the vector. Since our projection is 90 degrees, this is
    // guaranteed to give us the Z component of whatever face we are sampling from
    vec3 light_to_surface_ws_abs = abs(light_to_surface_ws);
    float face_local_z_depth = max(light_to_surface_ws_abs.x, max(light_to_surface_ws_abs.y, light_to_surface_ws_abs.z));

    // Determine the equivalent depth value we would expect to find in the cubemap. This is the z-portion of the
    // projection matrix. First we apply the projection and perspective divide. Then we convert from [-1, 1] range
    // to a [0, 1] range that the depth buffer expects
    //
    // Good info here:
    // https://stackoverflow.com/questions/10786951/omnidirectional-shadow-mapping-with-depth-cubemap
    float depth_value = (far+near) / (far-near) - (2*far*near)/(far-near)/face_local_z_depth;
    return (depth_value + 1.0) * 0.5;
}

// Return value: xy=UV, z=face index
// Based on https://www.gamedev.net/forums/topic/687535-implementing-a-cube-map-lookup-function/5337472/
vec3 cube_sample_to_uv_and_face_index(vec3 dir)
{
	vec3 dirAbs = abs(dir);
	float faceIndex;
	float ma;
	vec2 uv;

	if(dirAbs.z >= dirAbs.x && dirAbs.z >= dirAbs.y)
	{
		// Either -Z or +Z
		faceIndex = dir.z < 0.0 ? 5.0 : 4.0;
		ma = 0.5 / dirAbs.z;
		uv = vec2(dir.z < 0.0 ? -dir.x : dir.x, -dir.y);
	}
	else if(dirAbs.y >= dirAbs.x)
	{
	    // Either -Y or +Y
		faceIndex = dir.y < 0.0 ? 3.0 : 2.0;
		ma = 0.5 / dirAbs.y;
		uv = vec2(dir.x, dir.y < 0.0 ? -dir.z : dir.z);
	}
	else
	{
	    // Either -X or +X
		faceIndex = dir.x < 0.0 ? 1.0 : 0.0;
		ma = 0.5 / dirAbs.x;
		uv = vec2(dir.x < 0.0 ? dir.z : -dir.z, -dir.y);
	}

	return vec3(uv * ma + 0.5, faceIndex);
}

float do_calculate_percent_lit_cube(vec3 light_position_ws, vec3 light_position_vs, vec3 normal_vs, int index, float bias_multiplier) {
    // Determine the equivalent depth value that would come out of the shadow cubemap if this surface
    // was the sampled depth. We have 6 different view/projections but those are defined by the spec.
    // The only thing we need from outside the shader is the near/far plane of the projections
    float near_plane = per_view_data.shadow_map_cube_data[index].cube_map_projection_near_z;
    float far_plane = per_view_data.shadow_map_cube_data[index].cube_map_projection_far_z;
    vec3 light_to_surface_ws = in_position_ws.xyz - light_position_ws;

    // Tune with single-PCF
    // bias_angle_factor is high when the light angle is almost orthogonal to the normal
    vec3 surface_to_light_dir_vs = normalize(light_position_vs - in_position_vs);
    float bias_angle_factor = 1.0 - max(0, dot(in_normal_vs, surface_to_light_dir_vs));
    bias_angle_factor = pow(bias_angle_factor, 3);

    // TODO HERE: Want to rework bias to take input from outside the shader, also not sure if the depth bias +
    // slope scale depth bias is better or the max(MIN_BIAS, MAX_BIAS * slope_factor) is better
    // Good info here: https://digitalrune.github.io/DigitalRune-Documentation/html/3f4d959e-9c98-4a97-8d85-7a73c26145d7.htm
    //float bias = max(
    //    SHADOW_MAP_BIAS_MAX * bias_angle_factor * bias_angle_factor * bias_angle_factor,
    //    0.03 //SHADOW_MAP_BIAS_MIN * 0.5
    //) * POINT_LIGHT_SHADOW_MAP_BIAS_MULTIPLIER;

    //return bias_angle_factor;
    float bias = 0.0006 + (0.0060 * bias_angle_factor);

#ifdef PCF_CUBE_SAMPLE_1
    float depth_of_surface = calculate_cubemap_equivalent_depth(
        light_to_surface_ws,
        near_plane,
        far_plane
    );

    vec3 uv_and_face = cube_sample_to_uv_and_face_index(light_to_surface_ws);
    vec4 uv_min_uv_max = per_view_data.shadow_map_cube_data[index].uv_min_uv_max[int(uv_and_face.z)];

    // We allow some faces of cube maps to not be included in the shadow atlas. In this case, uv coordinates will be
    // -1 and we should early-out.
    // We set uv coordinates to -1 if this
    if (uv_min_uv_max.x < 0.0) {
        return 1.0;
    }

    // Convert the [0, 1] value to location in texture atlas
    vec2 uv_to_sample = mix(uv_min_uv_max.xy, uv_min_uv_max.zw, uv_and_face.xy);
    float shadow = texture(
        sampler2DShadow(shadow_map_atlas, smp_depth_nearest),
        vec3(
            uv_to_sample,
            depth_of_surface + bias
        )
    ).r;
#endif

#ifdef PCF_CUBE_SAMPLE_20
    vec3 sampleOffsetDirections[20] = vec3[]
    (
        vec3( 1,  1,  1), vec3( 1, -1,  1), vec3(-1, -1,  1), vec3(-1,  1,  1),
        vec3( 1,  1, -1), vec3( 1, -1, -1), vec3(-1, -1, -1), vec3(-1,  1, -1),
        vec3( 1,  1,  0), vec3( 1, -1,  0), vec3(-1, -1,  0), vec3(-1,  1,  0),
        vec3( 1,  0,  1), vec3(-1,  0,  1), vec3( 1,  0, -1), vec3(-1,  0, -1),
        vec3( 0,  1,  1), vec3( 0, -1,  1), vec3( 0, -1, -1), vec3( 0,  1, -1)
    );

    float shadow = 0.0;
    int samples = 20;
    float diskRadius = 0.01;
    //float diskRadius = (1.0 + (view_distance / far_plane)) / 25.0;

    for(int i = 0; i < samples; ++i)
    {
        vec3 offset = sampleOffsetDirections[i] * diskRadius;
        float depth_of_surface = calculate_cubemap_equivalent_depth(
            light_to_surface_ws + offset,
            near_plane,
            far_plane
        );

        vec3 uv_and_face = cube_sample_to_uv_and_face_index(light_to_surface_ws);
        vec4 uv_min_uv_max = per_view_data.shadow_map_cube_data[index].uv_min_uv_max[int(uv_and_face.z)];

        // We allow some faces of cube maps to not be included in the shadow atlas. In this case, uv coordinates will be -1
        if (uv_min_uv_max.x >= 0.0) {
            // Convert the [0, 1] value to location in texture atlas
            vec2 uv_to_sample = mix(uv_min_uv_max.xy, uv_min_uv_max.zw, uv_and_face.xy);
            shadow += texture(
                sampler2DShadow(shadow_map_atlas, smp_depth_nearest),
                vec3(
                    uv_to_sample,
                    depth_of_surface + bias
                )
            ).r;
        } else {
            shadow += 1.0;
        }
    }
    shadow /= float(samples);
#endif

#ifdef PCF_CUBE_SAMPLE_8
    float shadow  = 0.0;
    float samples = 2.0;
    float offset  = 0.05;
    for(float x = -offset; x < offset; x += offset / (samples * 0.5))
    {
        for(float y = -offset; y < offset; y += offset / (samples * 0.5))
        {
            for(float z = -offset; z < offset; z += offset / (samples * 0.5))
            {
                float depth_of_surface = calculate_cubemap_equivalent_depth(
                    light_to_surface_ws + vec3(x, y, z),
                    near_plane,
                    far_plane
                );

                vec3 uv_and_face = cube_sample_to_uv_and_face_index(light_to_surface_ws);
                vec4 uv_min_uv_max = per_view_data.shadow_map_cube_data[index].uv_min_uv_max[int(uv_and_face.z)];

                // We allow some faces of cube maps to not be included in the shadow atlas. In this case, uv coordinates will be -1
                if (uv_min_uv_max.x >= 0.0) {
                    // Convert the [0, 1] value to location in texture atlas
                    vec2 uv_to_sample = mix(uv_min_uv_max.xy, uv_min_uv_max.zw, uv_and_face.xy);
                    shadow += texture(
                        sampler2DShadow(shadow_map_atlas, smp_depth_nearest),
                        vec3(
                            uv_to_sample,
                            depth_of_surface + bias
                        )
                    ).r;
                } else {
                    shadow += 1.0;
                }
            }
        }
    }
    shadow /= (samples * samples * samples);
#endif

#ifdef PCF_CUBE_SAMPLE_64
    float shadow  = 0.0;
    //float bias    = 0.0002;
    float samples = 4.0;
    float offset  = 0.05;
    for(float x = -offset; x < offset; x += offset / (samples * 0.5))
    {
        for(float y = -offset; y < offset; y += offset / (samples * 0.5))
        {
            for(float z = -offset; z < offset; z += offset / (samples * 0.5))
            {
                float depth_of_surface = calculate_cubemap_equivalent_depth(
                    light_to_surface_ws + vec3(x, y, z),
                    near_plane,
                    far_plane
                );

                vec3 uv_and_face = cube_sample_to_uv_and_face_index(light_to_surface_ws);
                vec4 uv_min_uv_max = per_view_data.shadow_map_cube_data[index].uv_min_uv_max[int(uv_and_face.z)];

                // We allow some faces of cube maps to not be included in the shadow atlas. In this case, uv coordinates will be -1
                if (uv_min_uv_max.x >= 0.0) {
                    // Convert the [0, 1] value to location in texture atlas
                    vec2 uv_to_sample = mix(uv_min_uv_max.xy, uv_min_uv_max.zw, uv_and_face.xy);
                    shadow += texture(
                        sampler2DShadow(shadow_map_atlas, smp_depth_nearest),
                        vec3(
                            uv_to_sample,
                            depth_of_surface + bias
                        )
                    ).r;
                } else {
                    shadow += 1.0;
                }
            }
        }
    }
    shadow /= (samples * samples * samples);
#endif

    return shadow;
}

float calculate_percent_lit_cube(vec3 light_position_ws, vec3 light_position_vs, vec3 normal_vs, int index, float bias_multiplier) {
    if (index == -1) {
        return 1.0;
    }

    return do_calculate_percent_lit_cube(light_position_ws, light_position_vs, normal_vs, index, bias_multiplier);
}

//TODO: Not sure about surface to light dir for spot lights... don't think it will do anything except give slightly bad
// bias results though
float do_calculate_percent_lit(vec3 normal_vs, int index, float bias_multiplier) {
    // Determine the equiavlent depth value that would come out of the shadow map if this surface was
    // the sampled depth
    //  - [shadowmap view/proj matrix] * [surface position]
    //  - perspective divide
    //  - Convert XY's [-1, 1] range to [0, 1] UV coordinate range so we can sample the shadow map
    //  - Flip the Y, UV is +y down and NDC is +y up
    //  - Use the Z which represents the depth of the surface from the shadow map's projection's point of view
    //    It is [0, 1] range already so no adjustment needed
    vec4 shadow_map_pos = per_view_data.shadow_map_2d_data[index].shadow_map_view_proj * in_position_ws;
    vec3 projected = shadow_map_pos.xyz / shadow_map_pos.w;
    vec2 sample_location_uv = projected.xy * 0.5 + 0.5;
    sample_location_uv.y = 1.0 - sample_location_uv.y;
    vec2 uv_min = per_view_data.shadow_map_2d_data[index].uv_min;
    vec2 uv_max = per_view_data.shadow_map_2d_data[index].uv_max;
    sample_location_uv = mix(uv_min, uv_max, sample_location_uv);
    float depth_of_surface = projected.z;

    // Determine the direction of the light so we can apply more bias when light is near orthogonal to the normal
    // TODO: This is broken for spot lights. And is this mixing vs and ws data? Also we shouldn't consider normal maps here
    vec3 light_dir_vs = in_model_view * per_view_data.shadow_map_2d_data[index].shadow_map_light_dir;
    vec3 surface_to_light_dir_vs = -light_dir_vs;

    // Tune with single-PCF
    // bias_angle_factor is high when the light angle is almost orthogonal to the normal
    float bias_angle_factor = 1.0 - dot(normal_vs, surface_to_light_dir_vs);
    float bias = max(SHADOW_MAP_BIAS_MAX * bias_angle_factor * bias_angle_factor * bias_angle_factor, SHADOW_MAP_BIAS_MIN) * bias_multiplier;

    // This is used later in a compare to check if uv is inside uv_min and uv_max
    vec4 uv_min_max_compare = vec4(uv_min, -uv_max);

    // Non-PCF form (broken last time I tried it)
#ifdef PCF_DISABLED
    float percent_lit = 1.0;
    if (all(greaterThanEqual(vec4(sample_location_uv, -sample_location_uv), uv_min_max_compare))) {
        float distance_from_closest_object_to_light = texture(
            sampler2D(shadow_map_atlas, smp_depth_linear),
            sample_location_uv
        ).r;
        float percent_lit = depth_of_surface + bias < distance_from_closest_object_to_light ? 1.0 : 0.0;
    }
#endif

    // PCF form single sample
#ifdef PCF_SAMPLE_1
    float percent_lit = 1.0;
    if (all(greaterThanEqual(vec4(sample_location_uv, -sample_location_uv), uv_min_max_compare))) {
        percent_lit = texture(
            sampler2DShadow(shadow_map_atlas, smp_depth_linear),
            vec3(
                sample_location_uv,
                depth_of_surface + bias
            )
        ).r;
    }
#endif

    // PCF reasonable sample count
#ifdef PCF_SAMPLE_9
    float percent_lit = 0.0;
    vec2 texelSize = 1 / textureSize(sampler2DShadow(shadow_map_atlas, smp_depth_linear), 0);
    for(int x = -1; x <= 1; ++x)
    {
        for(int y = -1; y <= 1; ++y)
        {
            vec4 uv = vec4(sample_location_uv + vec2(x, y) * texelSize, 0.0, 0.0);
            uv.zw = -uv.xy;

            if (all(greaterThanEqual(uv, uv_min_max_compare))) {
                percent_lit += texture(
                    sampler2DShadow(shadow_map_atlas, smp_depth_linear),
                    vec3(
                        uv.xy,
                        depth_of_surface + bias
                    )
                ).r;
            } else {
                percent_lit += 1.0;
            }
        }
    }
    percent_lit /= 9.0;
#endif


    // PCF probably too many samples
#ifdef PCF_SAMPLE_25
    float percent_lit = 0.0;
    vec2 texelSize = 1 / textureSize(sampler2DShadow(shadow_map_atlas, smp_depth_linear), 0);
    for(int x = -2; x <= 2; ++x)
    {
        for(int y = -2; y <= 2; ++y)
        {
            vec4 uv = vec4(sample_location_uv + vec2(x, y) * texelSize, 0.0, 0.0);
            uv.zw = -uv.xy;

            if (all(greaterThanEqual(uv, uv_min_max_compare))) {
                percent_lit += texture(
                    sampler2DShadow(shadow_map_atlas, smp_depth_linear),
                    vec3(
                        uv.xy,
                        depth_of_surface + bias
                    )
                ).r;
            } else {
                percent_lit += 1.0;
            }
        }
    }
    percent_lit /= 25.0;
#endif

    return percent_lit;
}


float calculate_percent_lit(vec3 normal, int index, float bias_multiplier) {
    if (index == -1) {
        return 1.0;
    }

    return do_calculate_percent_lit(normal, index, bias_multiplier);
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

vec4 shade_diffuse_specular(
    vec3 surface_to_light_dir,
    vec3 surface_to_eye_dir,
    vec3 normal,
    vec4 light_color,
    float intensity // should include attenuation
) {
    vec4 diffuse = diffuse_light(surface_to_light_dir, normal, light_color) * intensity;
    vec4 specular = specular_light(surface_to_light_dir, surface_to_eye_dir, normal, light_color) * intensity;
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

// "Stable Geometric Specular Antialiasing with Projected-Space NDF Filtering"
// https://jcgt.org/published/0010/02/02/paper.pdf
// This is an adaptation of listing 3 which assumes a half-vector in tangent space.
// Our vectors are all in view space, but I think adding the derivatives of the half-vector
// and normal vector will produce similar results to calculating the half-vector in tangent space
// This implementation is appropriate for forward lighting
//
// It essentially increases roughness when the half-vector in tangent space changes quickly (probably
// because the normal is changing quickly.) This help reduce specular aliasing.
//
// Input:
//   n: Normal in any space
//   h: Half-vector in any space
//   roughness2: Roughness^2
float ForwardLightingNDFRoughnessFilter(vec3 n, vec3 h, float roughness2) {
    float SIGMA2 = 0.15915494;  // 1/2pi
    float KAPPA = 0.18;         // Max increase in roughness

    vec2 bounds = fwidth(h.xy) + fwidth(n.xy);
    float maxWidth = max(bounds.x, bounds.y);
    float kernelRoughness2 = 2.0 * SIGMA2 * (maxWidth * maxWidth);
    float clampedKernelRoughness2 = min(kernelRoughness2, KAPPA);
    return clamp(roughness2 + clampedKernelRoughness2, 0, 1);
}

// "Stable Geometric Specular Antialiasing with Projected-Space NDF Filtering"
// https://jcgt.org/published/0010/02/02/paper.pdf
// This is an adaptation of listing 5, almost verbatim. This is appropriate for deferred lighting
//
// It essentially increases roughness when the normal is changing quickly. This help reduce specular aliasing.
//
// Input:
//   normal: Normal in any space
//   roughness2: Roughness^2
float DeferredLightingNDFRoughnessFilter(vec3 normal, float roughness2, float ndf_filter_amount) {
    float SIGMA2 = 0.15915494;  // 1/2pi
    float KAPPA = 0.18;         // Max increase in roughness

    vec3 dndu = dFdx(normal);
    vec3 dndv = dFdy(normal);
    float kernelRoughness2 = 2.0 * SIGMA2 * (dot(dndu, dndu) + dot(dndv, dndv));
    float clampedKernelRoughness2 = min(kernelRoughness2, KAPPA);
    return clamp(roughness2 + clampedKernelRoughness2 * ndf_filter_amount, 0, 1);
}

//
// Normal distribution function approximates the relative surface area where microfacets are aligned to the halfway
// vector, producing specular-like results. (GGX/Trowbridge-Reitz)
//
// disney/epic remap alpha, squaring roughness as it produces better results
// https://cdn2.unrealengine.com/Resources/files/2013SiggraphPresentationsNotes-26915738.pdf
//
float ndf_ggx(
    vec3 n,
    vec3 h,
    float roughness_squared
) {
    // If we want to use the forward lighting roughness filter, we would do that here. We use the deferred because
    // quality is very similar, it's once per pixel instead of pixel*lights, and it will work fine if we switch to
    // deferred
    //float a = ForwardLightingNDFRoughnessFilter(n, h, roughness_squared);
    float a = roughness_squared;
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
    //v = normalize(v);
    //h = normalize(h);
    float v_dot_h = max(dot(v, h), 0.0);

    // approximation for pow(1 - v_dot_h, 5)
    // https://seblagarde.wordpress.com/2012/06/03/spherical-gaussien-approximation-for-blinn-phong-phong-and-fresnel/
    // https://cdn2.unrealengine.com/Resources/files/2013SiggraphPresentationsNotes-26915738.pdf
    // See https://google.github.io/filament/Filament.md.html for alternatives
    return fresnel_base + (1.0 - fresnel_base) * exp2((-5.55473 * v_dot_h - 6.98316) * v_dot_h);
}

vec3 shade_pbr(
    vec3 surface_to_light_dir_vs,
    vec3 surface_to_eye_dir_vs,
    vec3 normal_vs,
    vec3 F0,
    vec3 base_color,
    float roughness,
    float roughness_ndf_filtered_squared,
    float metalness,
    vec3 radiance
) {
    vec3 halfway_dir_vs = normalize(surface_to_light_dir_vs + surface_to_eye_dir_vs);

    float NDF = ndf_ggx(normal_vs, halfway_dir_vs, roughness_ndf_filtered_squared);
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

vec3 point_light_pbr(
    vec3 light_position_vs,
    vec3 light_color,
    float light_intensity,
    vec3 surface_to_eye_dir_vs,
    vec3 surface_position_vs,
    vec3 normal_vs,
    vec3 F0,
    vec3 base_color,
    float roughness,
    float roughness_ndf_filtered_squared,
    float metalness
) {
    // Get the distance to the light and normalize the surface_to_light direction. (Not
    // using normalize since we want the distance too)
    vec3 surface_to_light_dir_vs = light_position_vs - surface_position_vs;
    float distance = length(surface_to_light_dir_vs);
    surface_to_light_dir_vs = surface_to_light_dir_vs / distance;

    // Figure out the falloff of light intensity due to distance from light source
    float attenuation = 1.0 / (0.001 + (distance * distance));

    vec3 radiance = light_color * attenuation * light_intensity;

    return shade_pbr(
        surface_to_light_dir_vs,
        surface_to_eye_dir_vs,
        normal_vs,
        F0,
        base_color,
        roughness,
        roughness_ndf_filtered_squared,
        metalness,
        radiance
    );
}


vec3 spot_light_pbr(
    vec3 light_position_vs,
    vec3 light_color,
    float light_intensity,
    vec3 light_direction_vs,
    float light_spotlight_half_angle,
    vec3 surface_to_eye_dir_vs,
    vec3 surface_position_vs,
    vec3 normal_vs,
    vec3 F0,
    vec3 base_color,
    float roughness,
    float roughness_ndf_filtered_squared,
    float metalness
) {
    // Get the distance to the light and normalize the surface_to_light direction. (Not
    // using normalize since we want the distance too)
    vec3 surface_to_light_dir_vs = light_position_vs - surface_position_vs;
    float distance = length(surface_to_light_dir_vs);
    surface_to_light_dir_vs = surface_to_light_dir_vs / distance;

    // Figure out the falloff of light intensity due to distance from light source
    float attenuation = 1.0 / (0.001 + (distance * distance));

    // Figure out the falloff of light intensity around the projected cone of light
    float spotlight_direction_intensity = spotlight_cone_falloff(
        surface_to_light_dir_vs,
        light_direction_vs,
        light_spotlight_half_angle
    );

    float radiance = attenuation * light_intensity * spotlight_direction_intensity;

    if (radiance > 0.0) {
        return shade_pbr(
            surface_to_light_dir_vs,
            surface_to_eye_dir_vs,
            normal_vs,
            F0,
            base_color,
            roughness,
            roughness_ndf_filtered_squared,
            metalness,
            radiance * light_color
        );
    } else {
        return vec3(0.0);
    }
}

vec3 directional_light_pbr(
    DirectionalLight light,
    vec3 surface_to_eye_dir_vs,
    vec3 surface_position_vs,
    vec3 normal_vs,
    vec3 F0,
    vec3 base_color,
    float roughness,
    float roughness_ndf_filtered_squared,
    float metalness
) {
    vec3 surface_to_light_dir_vs = -light.direction_vs;

    vec3 radiance = light.color.rgb * light.intensity;

    return shade_pbr(
        surface_to_light_dir_vs,
        surface_to_eye_dir_vs,
        normal_vs,
        F0,
        base_color,
        roughness,
        roughness_ndf_filtered_squared,
        metalness,
        radiance
    );
}

//
// Basic non-pbr lighting
//
float attenuate_light_for_range(
    float light_range,
    float distance
) {
    // Full lighting until 75% away, then step down to no lighting
    return 1.0 - smoothstep(light_range * .75, light_range, distance);
}

vec3 iterate_point_and_spot_lights_all(
    vec3 surface_to_eye_vs,
    vec4 base_color,
    float metalness,
    float roughness,
    vec3 normal_vs,
    vec3 fresnel_base,
    float roughness_ndf_filtered_squared,
    uint light_cluster_index
) {
    vec3 total_light = vec3(0.0);
    for (uint light_index = 0; light_index < all_lights.light_count; ++light_index) {
        LightInList light = all_lights.data[light_index];
        if (dot(light.spotlight_direction_vs, light.spotlight_direction_vs) > 0.01) {
            // it has a valid direction, so treat as a spot light
            float light_surface_distance = distance(light.position_ws, in_position_ws.xyz);
            float range = light.range;
            if (light_surface_distance <= range) {
                float soft_falloff_factor = attenuate_light_for_range(range, light_surface_distance);

#ifndef DEBUG_RENDER_PERCENT_LIT
                vec3 pbr = soft_falloff_factor * spot_light_pbr(
                    light.position_vs,
                    light.color.rgb,
                    light.intensity,
                    light.spotlight_direction_vs,
                    light.spotlight_half_angle,
                    surface_to_eye_vs,
                    in_position_vs,
                    normal_vs,
                    fresnel_base,
                    base_color.rgb,
                    roughness,
                    roughness_ndf_filtered_squared,
                    metalness
                );
#else
                vec3 pbr = vec3(soft_falloff_factor);
#endif

                float percent_lit = 1.0;
                if (any(greaterThan(pbr, vec3(0.0)))) {
                    percent_lit = calculate_percent_lit(
                        normal_vs,
                        light.shadow_map,
                        SPOT_LIGHT_SHADOW_MAP_BIAS_MULTIPLIER
                    );
                }

                total_light += percent_lit * pbr;
            }

        } else {
            // Directionless lights are point lights

            float light_surface_distance = distance(light.position_ws, in_position_ws.xyz);
            float range = light.range;
            if (light_surface_distance <= range) {
                float soft_falloff_factor = attenuate_light_for_range(range, light_surface_distance);

#ifndef DEBUG_RENDER_PERCENT_LIT
                vec3 pbr = soft_falloff_factor * point_light_pbr(
                    light.position_vs,
                    light.color.rgb,
                    light.intensity,
                    surface_to_eye_vs,
                    in_position_vs,
                    normal_vs,
                    fresnel_base,
                    base_color.rgb,
                    roughness,
                    roughness_ndf_filtered_squared,
                    metalness
                );
#else
                vec3 pbr = vec3(soft_falloff_factor);
#endif

                float percent_lit = 1.0;
                if (any(greaterThan(pbr, vec3(0.0)))) {
                    percent_lit = calculate_percent_lit_cube(
                        light.position_ws,
                        light.position_vs,
                        normal_vs,
                        light.shadow_map,
                        1.0
                    );
                }

                total_light += percent_lit * pbr;
            }
        }
    }

    return total_light;
}

vec3 iterate_point_and_spot_lights_clustered(
    vec3 surface_to_eye_vs,
    vec4 base_color,
    float metalness,
    float roughness,
    vec3 normal_vs,
    vec3 fresnel_base,
    float roughness_ndf_filtered_squared,
    uint light_cluster_index
) {
    vec3 total_light = vec3(0.0);
    uint light_first = light_bin_output.data.offsets[light_cluster_index].first_light;
    uint light_last = light_first + light_bin_output.data.offsets[light_cluster_index].count;

    for (uint light_list_index = light_first; light_list_index < light_last; ++light_list_index) {
        uint light_index = light_bin_output.data.data[light_list_index];
        LightInList light = all_lights.data[light_index];
        if (dot(light.spotlight_direction_vs, light.spotlight_direction_vs) > 0.01) {
            // it has a valid direction, so treat as a spot light
            float light_surface_distance = distance(light.position_ws, in_position_ws.xyz);
            float range = light.range;
            if (light_surface_distance <= range) {
                float soft_falloff_factor = attenuate_light_for_range(range, light_surface_distance);

#ifndef DEBUG_RENDER_PERCENT_LIT
                vec3 pbr = soft_falloff_factor * spot_light_pbr(
                    light.position_vs,
                    light.color.rgb,
                    light.intensity,
                    light.spotlight_direction_vs,
                    light.spotlight_half_angle,
                    surface_to_eye_vs,
                    in_position_vs,
                    normal_vs,
                    fresnel_base,
                    base_color.rgb,
                    roughness,
                    roughness_ndf_filtered_squared,
                    metalness
                );
#else
                vec3 pbr = vec3(soft_falloff_factor);
#endif

                float percent_lit = 1.0;
                if (any(greaterThan(pbr, vec3(0.0)))) {
                    percent_lit = calculate_percent_lit(
                        normal_vs,
                        light.shadow_map,
                        SPOT_LIGHT_SHADOW_MAP_BIAS_MULTIPLIER
                    );
                }

                total_light += percent_lit * pbr;
            }
        } else {
            // Directionless lights are point lights
            float light_surface_distance = distance(light.position_ws, in_position_ws.xyz);
            float range = light.range;
            if (light_surface_distance <= range) {
                float soft_falloff_factor = attenuate_light_for_range(range, light_surface_distance);

#ifndef DEBUG_RENDER_PERCENT_LIT
                vec3 pbr = soft_falloff_factor * point_light_pbr(
                    light.position_vs,
                    light.color.rgb,
                    light.intensity,
                    surface_to_eye_vs,
                    in_position_vs,
                    normal_vs,
                    fresnel_base,
                    base_color.rgb,
                    roughness,
                    roughness_ndf_filtered_squared,
                    metalness
                );
#else
                vec3 pbr = vec3(soft_falloff_factor);
#endif

                float percent_lit = 1.0;
                if (any(greaterThan(pbr, vec3(0.0)))) {
                    percent_lit = calculate_percent_lit_cube(
                        light.position_ws,
                        light.position_vs,
                        normal_vs,
                        light.shadow_map,
                        1.0
                    );
                }

                total_light += percent_lit * pbr;
            }
        }
    }

    return total_light;
}

vec4 pbr_path(
    vec3 surface_to_eye_vs,
    vec4 base_color,
    vec4 emissive_color,
    float metalness,
    float roughness,
    vec3 normal_vs,
    uint light_cluster_index,
    float ambient_factor
) {
    // used in fresnel, non-metals use 0.04 and metals use the base color
    vec3 fresnel_base = vec3(0.04);
    fresnel_base = mix(fresnel_base, base_color.rgb, vec3(metalness));
    float roughness_ndf_filtered_squared = DeferredLightingNDFRoughnessFilter(normal_vs, roughness * roughness, per_view_data.ndf_filter_amount);

    vec3 total_light = vec3(0.0);
    if (per_view_data.use_clustered_lighting)
    {
        total_light = iterate_point_and_spot_lights_clustered(
            surface_to_eye_vs,
            base_color,
            metalness,
            roughness,
            normal_vs,
            fresnel_base,
            roughness_ndf_filtered_squared,
            light_cluster_index
        );
    } else {
        total_light = iterate_point_and_spot_lights_all(
            surface_to_eye_vs,
            base_color,
            metalness,
            roughness,
            normal_vs,
            fresnel_base,
            roughness_ndf_filtered_squared,
            light_cluster_index
        );
    }

    // directional Lights
    for (uint i = 0; i < per_view_data.directional_light_count; ++i) {

#ifndef DEBUG_RENDER_PERCENT_LIT
        vec3 pbr = directional_light_pbr(
           per_view_data.directional_lights[i],
           surface_to_eye_vs,
           in_position_vs,
           normal_vs,
           fresnel_base,
           base_color.rgb,
           roughness,
           roughness_ndf_filtered_squared,
           metalness
       );
#else
        vec3 pbr = vec3(1.0);
#endif

        float percent_lit = 1.0;
        if (any(greaterThan(pbr, vec3(0.0)))) {
            percent_lit = calculate_percent_lit(
                normal_vs,
                per_view_data.directional_lights[i].shadow_map,
                DIRECTIONAL_LIGHT_SHADOW_MAP_BIAS_MULTIPLIER
            );
        }

        total_light += percent_lit * pbr;
    }

    //
    // There are still issues here, not sure how alpha interacts and gamma looks terrible
    //
    vec3 ambient = per_view_data.ambient_light.rgb * base_color.rgb * ambient_factor;

    uint material_index = all_draw_data.draw_data[in_instance_index].material_index;
    MaterialDbEntry per_material_data = all_materials.materials[material_index];

    float alpha = 1.0;
    if (per_material_data.enable_alpha_blend) {
        alpha = base_color.a;
    } else if (per_material_data.enable_alpha_clip && base_color.a < per_material_data.alpha_threshold) {
        alpha = 0.0;
    }

#ifdef DEBUG_RENDER_PERCENT_LIT
    vec3 color = total_light;
#else
    vec3 color = ambient + total_light + emissive_color.rgb;
#endif
    return vec4(color, alpha);
}

uint get_light_cluster_index() {
    float NEAR_Z = 5.0;
    float FAR_Z = 10000.0;
    int X_BINS = 16;
    int Y_BINS = 8;
    int Z_BINS = 24;
    uint cluster_coord_x = min(uint((gl_FragCoord.x / per_view_data.viewport_width) * float(X_BINS)), (X_BINS - 1));
    uint cluster_coord_y = min(uint((1.0 - (gl_FragCoord.y / per_view_data.viewport_height)) * float(Y_BINS)), (Y_BINS - 1));

    float top = float(Z_BINS - 1) * log(-in_position_vs.z / NEAR_Z);
    float bottom = log(FAR_Z / NEAR_Z);
    uint cluster_coord_z = uint(clamp((top / bottom) + 1.0, 0, Z_BINS - 1));

    uint linear_index = X_BINS * Y_BINS * cluster_coord_z + X_BINS * cluster_coord_y + cluster_coord_x;
    return linear_index;
}

uint hash_light_list(uint light_cluster_index) {
    uint light_first = light_bin_output.data.offsets[light_cluster_index].first_light;
    uint light_last = light_first + light_bin_output.data.offsets[light_cluster_index].count;

    uint hash = 0x811c9dc5;

    for (uint light_list_index = light_first; light_list_index < light_last; ++light_list_index) {
        uint light_index = light_bin_output.data.data[light_list_index];
        hash = (hash ^ light_index) * 0x01000193;
    }

    return hash;
}

vec4 pbr_main() {
    uint material_index = all_draw_data.draw_data[in_instance_index].material_index;
    MaterialDbEntry per_material_data = all_materials.materials[material_index];

    // Sample the base color, if it exists
    vec4 base_color = per_material_data.base_color_factor;
    float ambient_factor = 1.0;
    uint light_cluster_index = get_light_cluster_index();

#ifdef PBR_TEXTURES
    if (per_material_data.color_texture != -1) {
        vec4 sampled_color = texture(sampler2D(all_material_textures[per_material_data.color_texture], smp), in_uv, per_view_data.mip_bias);
        if (per_material_data.base_color_texture_has_alpha_channel) {
            base_color *= sampled_color;
        } else {
            base_color = vec4(base_color.rgb * sampled_color.rgb, base_color.a);
        }
    }

    float screen_coord_x = (gl_FragCoord.x / float(per_view_data.viewport_width));
    float screen_coord_y = ((gl_FragCoord.y / float(per_view_data.viewport_height)));
    ambient_factor = texture(sampler2D(ssao_texture, smp), vec2(screen_coord_x, screen_coord_y)).r;
#endif

    // Sample the emissive color, if it exists
    vec4 emissive_color = vec4(per_material_data.emissive_factor, 1);

#ifdef PBR_TEXTURES
    if (per_material_data.emissive_texture != -1) {
        emissive_color *= texture(sampler2D(all_material_textures[per_material_data.emissive_texture], smp), in_uv, per_view_data.mip_bias);
    }
#endif

    // Sample metalness/roughness
    float metalness = per_material_data.metallic_factor;
    float roughness = per_material_data.roughness_factor;

#ifdef PBR_TEXTURES
    if (per_material_data.metallic_roughness_texture != -1) {
        vec4 sampled = texture(sampler2D(all_material_textures[per_material_data.metallic_roughness_texture], smp), in_uv, per_view_data.mip_bias);
        metalness *= sampled.b;
        roughness *= sampled.g;
    }
#endif

    metalness = clamp(metalness, 0, 1);
    roughness = clamp(roughness, 0, 1);

    // Calculate the normal (use the normal map if it exists)
    vec3 normal_vs;

#ifdef PBR_TEXTURES
    if (per_material_data.normal_texture != -1) {
        mat3 tbn = mat3(in_tangent_vs, in_binormal_vs, in_normal_vs);
        normal_vs = normal_map(
            per_material_data.normal_texture,
            tbn,
            //normal_texture,
            //smp,
            in_uv
        ).xyz;
    } else {
        normal_vs = normalize(vec4(in_normal_vs, 0)).xyz;
    }
#else
    normal_vs = normalize(vec4(in_normal_vs, 0)).xyz;
#endif

    //TOOD: AO

    vec3 eye_position_vs = vec3(0, 0, 0);
    vec3 surface_to_eye_vs = normalize(eye_position_vs - in_position_vs);

    vec4 out_color = pbr_path(
        surface_to_eye_vs,
        base_color,
        emissive_color,
        metalness,
        roughness,
        normal_vs,
        light_cluster_index,
        ambient_factor
    );

    // LIGHT COUNT
    //uint light_count = light_bin_output.data.offsets[get_light_cluster_index()].count;
    //out_color = vec4(vec3(light_count / 32.0), 1.0);
    //out_color = out_color + vec4(clamp(light_count / 16.0, 0.0, 1.0)/4.0, 0.0, clamp((16-light_count) / 16.0, 0.0, 1.0)/4.0, 1.0);

    // CLUSTER INDEX
    //uint hash = hash_light_list(get_light_cluster_index());
    //out_color = vec4(float((hash & 0xFF000000)>>24)/255.0, float((hash & 0x00FF0000)>>16)/255.0, float((hash & 0x0000FF00)>>8)/255.0, 1.0);

    //out_color = vec4(vec3(dot(normal_vs, -in_shadow_map_light_dir_vs)), 1.0);
    //out_color = vec4(metalness);

    //out_color = vec4(calculate_percent_lit(normal_vs), 1.0);
    //out_color = vec4(normal_vs, 1.0);
    //out_color = vec4(in_normal_vs, 1.0);
    //out_color = vec4(in_tangent_vs, 1.0);
    //out_color = vec4(in_binormal_vs, 1.0);
    //out_color = vec4(normal_vs, 1.0);
    //out_color = vec4(vec3(ambient_factor), 1.0);
    //out_color = vec4(in_uv.x, in_uv.y, 0.0, 1.0);

    return out_color;
}