override block_light_warmth: f32 = 1.15;
override ambient_contrast: f32 = 1.2;

fn material(input: MaterialInput) -> Surface {
    let raw_light = input.lightmap;
    let contrast_light = pow(raw_light, vec3<f32>(ambient_contrast));

    let block_level = input.light.x;
    let warm_tint = mix(vec3<f32>(1.0), vec3<f32>(1.2, 0.85, 0.55), block_level * (block_light_warmth - 1.0));
    
    let lit_color = input.albedo * input.tint * vec4<f32>(contrast_light * warm_tint, 1.0);
    
    let boosted_rgb = select(lit_color.rgb, lit_color.rgb * 1.08, input.tint.g > input.tint.r);
    
    return Surface(vec4<f32>(boosted_rgb, lit_color.a));
}
