#version 450

#include "../util/color.glsl"

// @[export]
layout (set = 0, binding = 0) uniform texture2D history_tex;
// @[export]
layout (set = 0, binding = 1) uniform texture2D current_tex;
// @[export]
layout (set = 0, binding = 2) uniform texture2D velocity_tex;
// @[export]
layout (set = 0, binding = 3) uniform texture2D depth_tex;

// @[immutable_samplers([
//     (
//         mag_filter: Nearest,
//         min_filter: Nearest,
//         mip_map_mode: Nearest,
//         address_mode_u: ClampToEdge,
//         address_mode_v: ClampToEdge,
//         address_mode_w: ClampToEdge,
//     )
// ])]
layout (set = 0, binding = 4) uniform sampler smp_nearest;

// @[immutable_samplers([
//     (
//         mag_filter: Linear,
//         min_filter: Linear,
//         mip_map_mode: Nearest,
//         address_mode_u: ClampToEdge,
//         address_mode_v: ClampToEdge,
//         address_mode_w: ClampToEdge,
//     )
// ])]
layout (set = 0, binding = 5) uniform sampler smp_bilinear;

// @[export]
// @[internal_buffer]
layout (set = 0, binding = 6) uniform Config {
    mat4 current_view_proj_inv;
    mat4 previous_view_proj;
    vec2 jitter_amount;
    bool has_history_data;
    bool enable_side_by_side_debug_view;
    float history_weight;
    float history_weight_velocity_adjust_multiplier;
    float history_weight_velocity_adjust_max;
    uint viewport_width;
    uint viewport_height;
} config;

layout (location = 0) in vec2 inUV;

layout (location = 0) out vec4 out_image;

float draw_line(vec2 p1, vec2 p2, vec2 x, float width) {
    //
    // Project a line from p1 to x onto the normalized vector from p1 to p2. This gives us the point on the line nearest
    // to x. The distance from x to that point is the distance from the line.
    //
    vec2 p1_to_x = x - p1;
    vec2 line = p2 - p1;
    vec2 line_dir = normalize(line);
    vec2 p1_to_nearest_on_line = dot(p1_to_x, line_dir) * line_dir;
    vec2 nearest_on_line = p1 + p1_to_nearest_on_line;

    //
    // This is a segment, not a line, so we need to clip the ends of the line at p1 and p2
    //
    float dist_nearest_from_p2 = length(p2 - nearest_on_line);
    float max_dist_from_p = length(line);
    if (dist_nearest_from_p2 > max_dist_from_p) {
        return 0.0;
    }

    float dist_nearest_from_p1 = length(p1 - nearest_on_line);
    if (dist_nearest_from_p1 > max_dist_from_p) {
        return 0.0;
    }

    width /= 2.0;
    float width_min = width * 0.95;
    float width_max = width * 1.05;

    // Return 1 for any value < width. Return 0 for any value > width + 1. Linear interpolate between that point.
    float dist_from_line = length(x - nearest_on_line);
    return clamp(1.0 - ((dist_from_line - width_min) / (width_max - width_min)), 0.0, 1.0);
}

/////////////////////////////////////////////////////////////////////////////////////
// sample_history_catmull_rom used under MIT License - Copyright(c) 2019 MJP
// Source: https://gist.github.com/TheRealMJP/c83b8c0f46b63f3a88a5986f4fa982b1
// (Some minor changes made)
// Samples a texture with Catmull-Rom filtering, using 9 texture fetches instead of 16.
// See http://vec3.ca/bicubic-filtering-in-fewer-taps/ for more details
/////////////////////////////////////////////////////////////////////////////////////
vec3 sample_history_catmull_rom(vec2 uv, vec2 texel_size)
{
    // We're going to sample a a 4x4 grid of texels surrounding the target UV coordinate. We'll do this by rounding
    // down the sample location to get the exact center of our "starting" texel. The starting texel will be at
    // location [1, 1] in the grid, where [0, 0] is the top left corner.
    vec2 samplePos = uv / texel_size;
    vec2 texPos1 = floor(samplePos - 0.5f) + 0.5f;

    // Compute the fractional offset from our starting texel to our original sample location, which we'll
    // feed into the Catmull-Rom spline function to get our filter weights.
    vec2 f = samplePos - texPos1;

    // Compute the Catmull-Rom weights using the fractional offset that we calculated earlier.
    // These equations are pre-expanded based on our knowledge of where the texels will be located,
    // which lets us avoid having to evaluate a piece-wise function.
    vec2 w0 = f * (-0.5f + f * (1.0f - 0.5f * f));
    vec2 w1 = 1.0f + f * f * (-2.5f + 1.5f * f);
    vec2 w2 = f * (0.5f + f * (2.0f - 1.5f * f));
    vec2 w3 = f * f * (-0.5f + 0.5f * f);

    // Work out weighting factors and sampling offsets that will let us use bilinear filtering to
    // simultaneously evaluate the middle 2 samples from the 4x4 grid.
    vec2 w12 = w1 + w2;
    vec2 offset12 = w2 / (w1 + w2);

    // Compute the final UV coordinates we'll use for sampling the texture
    vec2 texPos0 = texPos1 - 1.0f;
    vec2 texPos3 = texPos1 + 2.0f;
    vec2 texPos12 = texPos1 + offset12;

    texPos0 *= texel_size;
    texPos3 *= texel_size;
    texPos12 *= texel_size;

    vec3 result = vec3(0.0f, 0.0f, 0.0f);

    result += textureLod(sampler2D(history_tex, smp_bilinear), vec2(texPos0.x, texPos0.y), 0.0f).xyz * w0.x * w0.y;
    result += textureLod(sampler2D(history_tex, smp_bilinear), vec2(texPos12.x, texPos0.y), 0.0f).xyz * w12.x * w0.y;
    result += textureLod(sampler2D(history_tex, smp_bilinear), vec2(texPos3.x, texPos0.y), 0.0f).xyz * w3.x * w0.y;

    result += textureLod(sampler2D(history_tex, smp_bilinear), vec2(texPos0.x, texPos12.y), 0.0f).xyz * w0.x * w12.y;
    result += textureLod(sampler2D(history_tex, smp_bilinear), vec2(texPos12.x, texPos12.y), 0.0f).xyz * w12.x * w12.y;
    result += textureLod(sampler2D(history_tex, smp_bilinear), vec2(texPos3.x, texPos12.y), 0.0f).xyz * w3.x * w12.y;

    result += textureLod(sampler2D(history_tex, smp_bilinear), vec2(texPos0.x, texPos3.y), 0.0f).xyz * w0.x * w3.y;
    result += textureLod(sampler2D(history_tex, smp_bilinear), vec2(texPos12.x, texPos3.y), 0.0f).xyz * w12.x * w3.y;
    result += textureLod(sampler2D(history_tex, smp_bilinear), vec2(texPos3.x, texPos3.y), 0.0f).xyz * w3.x * w3.y;

    return max(result, 0.0f);
}

/////////////////////////////////////////////////////////////////////////////////////
// clip_aabb used under MIT License - Copyright(c) 2015 Playdead
// Source: https://github.com/playdeadgames/temporal/blob/master/Assets/Shaders/TemporalReprojection.shader
// (Some minor changes made)
/////////////////////////////////////////////////////////////////////////////////////
vec3 clip_aabb(vec3 aabb_min, vec3 aabb_max, vec3 history_color, vec3 average)
{
    const float EPSILON = 0.000001;
	#if 0 // OPTIMIZATIONS
		// note: only clips towards aabb center (but fast!)
		vec3 p_clip = 0.5 * (aabb_max + aabb_min);
		vec3 e_clip = 0.5 * (aabb_max - aabb_min) + EPSILON;

		vec3 v_clip = history_color - p_clip;
		vec3 v_unit = v_clip.xyz / e_clip;
		vec3 a_unit = abs(v_unit);
		float ma_unit = max(a_unit.x, max(a_unit.y, a_unit.z));

		if (ma_unit > 1.0)
			return p_clip + v_clip / ma_unit;
		else
			return history_color;// point inside aabb
	#else
		vec3 r = history_color - average;
		vec3 rmax = aabb_max - average.xyz;
		vec3 rmin = aabb_min - average.xyz;

		const float eps = EPSILON;

		if (r.x > rmax.x + eps)
			r *= (rmax.x / r.x);
		if (r.y > rmax.y + eps)
			r *= (rmax.y / r.y);
		if (r.z > rmax.z + eps)
			r *= (rmax.z / r.z);

		if (r.x < rmin.x - eps)
			r *= (rmin.x / r.x);
		if (r.y < rmin.y - eps)
			r *= (rmin.y / r.y);
		if (r.z < rmin.z - eps)
			r *= (rmin.z / r.z);

		return average + r;
	#endif
}

// GENERAL STEPS:
// 1. Gather color data around the current pixel, we will use it later to detect/fix large discrepencies with history
// 2. Velocity Sample
//    - The velocity texture may have aliasing, so do a 3x3 search and take the closest Z depth
// 3. Lookup previous color value
//    - May use a better sampling method than bilinear to avoid accumulating blurriness across frames
// 4. If the previous sample is "far away" from current sample, find "nearest" color that is not too "far away" from the
//    current color
//    - This can be approximated with an AABB, or more sophisticated (but still fast) methods ("variance clipping")
// 5. Determine weighting of previous/current samples
//    - Can vary weighting seeing how similar neighboring pixels are
//    - Can vary based on luminance (i.e. multiplay by 1/(1+Luminance(x)) to penalize bright pixels). This is an
//      approximation of tonemapping, so it means we're doing our weighting in a color space closer to final output.
void main() {
    out_image = vec4(0.0, 0.0, 0.0, 1.0);

    //
    // SIDE BY SIDE TEMP DEMO
    //
    vec2 in_uv = inUV;
    if (config.enable_side_by_side_debug_view) {
        out_image += draw_line(vec2(0.5, 0.0), vec2(0.5, 1.0), inUV, 0.002);
        if (in_uv.x < 0.5) {
            in_uv.x += 0.25;
            vec3 current_color = texture(sampler2D(current_tex, smp_nearest), in_uv).rgb;
            out_image.xyz += current_color;
            return;
        }
        in_uv.x -= 0.25;
    }

    //
    // If our history texture is invalid, just pass the color texture through and bail
    //
    if (!config.has_history_data) {
        vec3 current_color = texture(sampler2D(current_tex, smp_nearest), in_uv).rgb;
        out_image.xyz += current_color;
        return;
    }

    //
    // 1. Collect stats for variance clipping by sampling neighboring color values
    //
    const int COLOR_SAMPLE_RADIUS = 1;
    vec3 color_sum = vec3(0.0);
    float color_weight = 0.0;
    vec3 m1 = vec3(0.0);
    vec3 m2 = vec3(0.0);
    float m_weight = 0.0;

    // We assume all textures are the same size
    vec2 texture_size = textureSize(sampler2D(current_tex, smp_nearest), 0);
    vec2 texel_size = 1.0 / texture_size;
    vec3 current_color;
    for (int y = -COLOR_SAMPLE_RADIUS; y <= COLOR_SAMPLE_RADIUS; ++y)
    {
        for (int x = -COLOR_SAMPLE_RADIUS; x <= COLOR_SAMPLE_RADIUS; ++x) {
            vec2 sample_uv = clamp(in_uv + vec2(x, y) * texel_size, 0.0, 1.0);
            vec3 color = texture(sampler2D(current_tex, smp_nearest), sample_uv).rgb;

            float luminance = rgb_to_luminosity(color);

            //TODO: Consider additional filter based on distance from sample point
            float weight = 1.0 / (1.0 + luminance);

            color_sum += weight * color;
            color_weight += weight;

            m1 += color;
            m2 += color * color;
            m_weight += 1.0;

            if (x == 0 && y == 0) {
                current_color = color;
            }
        }
    }

    //
    // 2. Get velocity - with 1px dilation favoring closest fragment. The dialation ensures we grab the closest value
    //    even if the aliasing in the input would make us miss it on some pixels
    //
    float depth;
    vec2 velocity_ndc;
    {
        const int VELOCITY_SAMPLE_RADIUS = 1;
        float closest_depth = -1.0;
        vec2 closest_velocity_ndc = vec2(0.0);
        for (int y = -VELOCITY_SAMPLE_RADIUS; y <= VELOCITY_SAMPLE_RADIUS; ++y)
        {
            for (int x = -VELOCITY_SAMPLE_RADIUS; x <= VELOCITY_SAMPLE_RADIUS; ++x) {
                vec2 sample_uv = clamp(in_uv + vec2(x, y) * texel_size, 0.0, 1.0);
                vec2 v = texture(sampler2D(velocity_tex, smp_nearest), sample_uv).rg;
                float d = texture(sampler2D(depth_tex, smp_nearest), sample_uv).r;
                if (d > closest_depth) {
                    closest_depth = d;
                    closest_velocity_ndc = v;
                }
            }
        }

        depth = closest_depth;
        velocity_ndc = closest_velocity_ndc;
    }

    if (depth <= 0.0) {
        //
        // Infinitely far away - assume it can't alias (i.e. a skybox or something like that). Snap it to the color
        // sample
        //
        out_image.xyz += current_color;
        return;
    } else if (velocity_ndc.x > 9000000.0 && velocity_ndc.y > 9000000.0) {
        //
        // There is no velocity data, reproject using our current and previous view/projection matrices
        //
        vec2 viewport_size = vec2(config.viewport_width, config.viewport_height);
        vec2 fragcoord_ndc = (gl_FragCoord.xy / viewport_size) * 2.0 - 1.0;
        fragcoord_ndc.y *= -1.0;
        vec4 new_position_ndc = vec4(fragcoord_ndc, depth, 1.0);
        vec4 position_ws = config.current_view_proj_inv * new_position_ndc;
        position_ws /= position_ws.w;
        vec4 previous_position_ndc = config.previous_view_proj * vec4(position_ws.xyz, 1.0);
        previous_position_ndc /= previous_position_ndc.w;
        velocity_ndc = fragcoord_ndc - previous_position_ndc.xy;
    }

    //
    // 3. Get history color (default to current color if outside the bounds of history texture)
    //
    vec3 history_color = current_color;
    vec2 history_sample_uv = inUV - (velocity_ndc * vec2(0.5, -0.5)); // ndc -> uv
    if (history_sample_uv.x <= 1.0 && history_sample_uv.x >= 0.0 && history_sample_uv.y <= 1.0 && history_sample_uv.y >= 0.0) {
        // catmull-rom filtering reduces accumulation of blur
        history_color = sample_history_catmull_rom(history_sample_uv, texel_size);
        //history_color = texture(sampler2D(history_tex, smp), history_sample_uv).rgb;
    }

    //
    // (debug) Calculate an accuracy value
    //
    //vec3 color_history_diff = abs(history_color - current_color);
    //float error = sqrt(dot(color_history_diff, color_history_diff));

    //
    // 4. Do variance clipping
    //
    vec3 mu = m1 / m_weight;
    vec3 sigma = sqrt(abs(m2 / m_weight - mu * mu));
    const float VARIANCE_CLIP_GAMMA = 1.0;
    vec3 min_c = mu - VARIANCE_CLIP_GAMMA * sigma;
    vec3 max_c = mu + VARIANCE_CLIP_GAMMA * sigma;
    history_color = clip_aabb(min_c, max_c, history_color, mu);

    //
    // 5. Weighting/Blending
    //
    // does velocity need to be normalized by viewport size?
    float current_weight = config.history_weight + min(length(velocity_ndc) * config.history_weight_velocity_adjust_multiplier, config.history_weight_velocity_adjust_max);
    float history_weight = 1.0 - current_weight;

    current_weight *= 1.0 / (1.0 + rgb_to_luminosity(current_color));
    history_weight *= 1.0 / (1.0 + rgb_to_luminosity(history_color));

    vec3 blended_color = (current_weight * current_color + history_weight * history_color) / (current_weight + history_weight);
    //blended_color.r = error;
    out_image += vec4(blended_color, 1.0);
}
