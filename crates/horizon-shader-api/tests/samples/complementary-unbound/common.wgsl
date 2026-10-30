fn luminance(color: vec3<f32>) -> f32 {
    return dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
}

fn aces_filmic(x: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

fn soft_tonemap(x: vec3<f32>) -> vec3<f32> {
    return x / (x + vec3<f32>(1.0));
}

fn adjust_saturation(color: vec3<f32>, sat: f32) -> vec3<f32> {
    let grey = luminance(color);
    return mix(vec3<f32>(grey), color, sat);
}
