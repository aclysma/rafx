#version 450
#extension GL_ARB_separate_shader_objects : enable

// @[export]
layout (set = 0, binding = 0) uniform texture2D tex;

// @[immutable_samplers([
//         (
//             mag_filter: Nearest,
//             min_filter: Nearest,
//             mip_map_mode: Linear,
//             address_mode_u: ClampToEdge,
//             address_mode_v: ClampToEdge,
//             address_mode_w: ClampToEdge,
//         )
// ])]
layout (set = 0, binding = 1) uniform sampler smp;

layout (location = 0) in vec2 inUV;

layout (location = 0) out vec4 out_sdr;
layout (location = 1) out vec4 out_bloom;

void main()
{
    // Extraction of a single pixel can easily lead to "fireflies", where single pixels of extreme
    // values cause flickering splashes of bloom that are very distracting. Here we sample a few
    // pixels in an area and do a weighted average, reducing the weight for pixels with extreme
    // values. General approach described here:
    // https://catlikecoding.com/unity/tutorials/custom-srp/hdr/

    // Extract from matching pixel and 4 surrounding pixels
    //TODO: If we downsample to half size, we can effectively 4x the sample count here
    vec2 offsets[] = {
		vec2(-1.0, -1.0),
		vec2(-1.0, 1.0),
		vec2(1.0, -1.0),
		vec2(1.0, 1.0)
	};

    // First iteration, fetch offset (0, 0)
    vec3 c = texture(sampler2D(tex, smp), inUV).rgb;
    out_sdr = vec4(c, 1.0);
    float luminance = dot(c, vec3(0.2126, 0.7152, 0.0722));
    float weight = 1.0 / (luminance + 1.0);
    vec3 color = c * weight;
    float weightSum = weight;

    vec2 tex_offset = 1.0 / textureSize(sampler2D(tex, smp), 0);
	for (int i = 0; i < 4; ++i) {
        vec3 c = texture(sampler2D(tex, smp), inUV + (offsets[i] * tex_offset)).rgb;

        // Constant from https://en.wikipedia.org/wiki/Relative_luminance
        luminance = dot(c, vec3(0.2126, 0.7152, 0.0722));

        float weight = 1.0 / (luminance + 1.0);
        color += c * weight;
        weightSum += weight;
	}

	color /= weightSum;

    //TODO: Do we really want to check luminance here?
    luminance = dot(color, vec3(0.2126, 0.7152, 0.0722));
    if (luminance > 1.0f) {
        out_bloom = vec4(color, 1.0);
    } else {
        out_bloom = vec4(0.0, 0.0, 0.0, 1.0);
    }
}
