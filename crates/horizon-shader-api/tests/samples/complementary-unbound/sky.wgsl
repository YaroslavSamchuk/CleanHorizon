override sky_glow: f32 = 0.6;

fn sky(input: SkyInput) -> vec4<f32> {
    let view_dir = normalize(input.direction);
    let sun_dir = frame.sun_direction;
    let moon_dir = frame.moon_direction;
    
    let cos_sun = max(dot(view_dir, sun_dir), 0.0);
    let sun_halo = pow(cos_sun, 12.0) * vec3<f32>(1.0, 0.65, 0.3) * sky_glow;
    let sun_disc = pow(cos_sun, 256.0) * vec3<f32>(1.0, 0.9, 0.7) * 0.8;
    
    let cos_moon = max(dot(view_dir, moon_dir), 0.0);
    let moon_halo = pow(cos_moon, 16.0) * vec3<f32>(0.4, 0.6, 0.9) * (sky_glow * 0.5);
    
    let horizon_dist = 1.0 - abs(view_dir.y);
    let sunset_weight = clamp(1.0 - abs(sun_dir.y) * 4.0, 0.0, 1.0);
    let twilight_color = vec3<f32>(0.95, 0.45, 0.25) * pow(horizon_dist, 4.0) * sunset_weight * 0.75;
    
    let weather_dim = 1.0 - (frame.rain * 0.4 + frame.thunder * 0.3);
    let final_sky = (input.color.rgb + sun_halo + sun_disc + moon_halo + twilight_color) * weather_dim;
    
    return vec4<f32>(final_sky, input.color.a);
}
