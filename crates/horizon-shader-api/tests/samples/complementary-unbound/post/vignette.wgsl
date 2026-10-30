override vignette_strength: f32 = 0.25;

fn post(input: PostInput) -> vec4<f32> {
    let uv = input.uv;
    let d = distance(uv, vec2<f32>(0.5, 0.5));
    let vig = smoothstep(0.75, 0.25, d * (1.0 + vignette_strength * 2.0));
    let final_rgb = input.color.rgb * mix(1.0 - vignette_strength, 1.0, vig);
    
    return vec4<f32>(final_rgb, input.color.a);
}
