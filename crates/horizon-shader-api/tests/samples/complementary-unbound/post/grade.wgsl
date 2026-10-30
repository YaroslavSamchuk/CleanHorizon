override saturation: f32 = 1.2;
override tonemap: u32 = 1u;
override warmth: f32 = 1.05;

fn post(input: PostInput) -> vec4<f32> {
    var c = input.color.rgb;
    
    c = c * vec3<f32>(warmth, 1.0, 2.0 - warmth);
    
    if (tonemap == 1u) {
        c = aces_filmic(c);
    } else if (tonemap == 2u) {
        c = soft_tonemap(c);
    }
    
    c = adjust_saturation(c, saturation);
    
    return vec4<f32>(c, input.color.a);
}
