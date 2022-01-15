
vec4 add_jitter(vec4 clip_position, vec2 jitter_amount) {
    clip_position.xy += jitter_amount * clip_position.w;
    return clip_position;
}

vec4 subtract_jitter(vec4 clip_position, vec2 jitter_amount) {
    clip_position.xy -= jitter_amount * clip_position.w;
    return clip_position;
}
