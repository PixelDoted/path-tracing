#define_import_path path_tracing::raytrace

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_render::{view::View, globals::Globals}
#import bevy_render::maths::{PI, HALF_PI}
#import bevy_pbr::lighting;
#import bevy_pbr::pbr_functions;

#import path_tracing::math::{EPSILON, U32_MAX}

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<uniform> globals: Globals;
@group(0) @binding(2) var<uniform> settings: Settings;

#import path_tracing::query::{Ray, HitRecord, hit_record, hit_all, objects};

@group(3) @binding(0) var<storage> materials: array<Material>;
@group(3) @binding(1) var<storage> textures: array<Texture>;
@group(3) @binding(2) var<storage> texture_data: array<f32>;

// ---- Binding Data ----

struct Settings {
    bounces: u32,
    samples: u32,
    fov: f32,
    sky_color: vec3<f32>,
}

struct Material {
    albedo: vec3<f32>,
    albedo_texture: u32,
    emissive: vec3<f32>,
    emissive_texture: u32,
    roughness: f32,
    metallic: f32,
    metallic_roughness_texture: u32,
    reflectance: f32,
    normal_map_texture: u32,
}

struct Texture {
    width: u32,
    height: u32,
    offset: u32,
    format: u32,
}

// --- Runtime Data ----

struct BRDFOutput {
    ray_dir: vec3<f32>,
    color: vec3<f32>,
}

var<private> rng_state: vec3<u32>;

// ---- Random ----

// http://www.jcgt.org/published/0009/03/02/
fn pcg3d() -> vec3<u32> {
    var v = rng_state;
    v = v * 1664525u + 1013904223u;

    v.x += v.y * v.z;
    v.y += v.z * v.x;
    v.z += v.x * v.y;

    v.x ^= v.x >> 16u;
    v.y ^= v.y >> 16u;
    v.z ^= v.z >> 16u;

    v.x += v.y * v.z;
    v.y += v.z * v.x;
    v.z += v.x * v.y;

    rng_state = v;
    return v;
}

fn rand() -> vec3<f32> {
    return abs(fract(vec3<f32>(pcg3d()) / 3141.592653589793));
}

fn rand_unit() -> vec3<f32> {
    return normalize(vec3<f32>(pcg3d()) / 3141.592653589793);
}

fn cosine_sample() -> vec3<f32> {
    let rng = rand();
    let phi = 2 * PI * rng.x;
    let sqr_sin_theta = rng.y;
    let sin_theta = sqrt(sqr_sin_theta);
    let cos_theta = sqrt(1.0 - sqr_sin_theta);
    return vec3<f32>(sin_theta * cos(phi), sin_theta * sin(phi), cos_theta);
}

fn rng_setup(uv: vec2<f32>) {
    rng_state = vec3<u32>(u32(uv.x), u32(uv.y), u32(uv.x) ^ u32(uv.y));
}

// ---- Helper ----
fn hugues_moller(n: vec3<f32>) -> mat3x3<f32> {
    let a = abs(n);
    var t = vec3<f32>(0);
    if a.x <= a.y && a.x <= a.z {
        t = vec3<f32>(0, -n.z, n.y);
    } else if a.y <= a.x && a.y <= a.z {
        t = vec3<f32>(-n.z, 0, n.x);
    } else {
        t = vec3<f32>(-n.y, n.x, 0);
    }
    t = normalize(t);
                    
    let b = normalize(cross(n, t));
    return mat3x3<f32>(t, b, n);
}

// ---- Texture ----

fn sample_texture(idx: u32, u: f32, v: f32) -> vec3<f32> {
    let texture = textures[idx];
    let x = u * f32(texture.width);
    let y = v * f32(texture.height);
    let i = texture.offset + (u32(x) + u32(y) * texture.height) * texture.format;

    switch (texture.format) {
        case 1u: {
            let r = texture_data[i];
            return vec3<f32>(r);
        }
        case 2u: {
            let r = texture_data[i];
            let g = texture_data[i + 1];
            return vec3<f32>(r, g, 0.0);
        }
        case 3u: {
            let r = texture_data[i];
            let g = texture_data[i + 1];
            let b = texture_data[i + 2];
            return vec3<f32>(r, g, b);
        }
        case 4u: {
            let r = texture_data[i];
            let g = texture_data[i + 1];
            let b = texture_data[i + 2];
            let a = texture_data[i + 3];
            return vec3<f32>(r, g, b) * a;
        }
        default: {
            return vec3<f32>(1.0);
        }
    }
}

// ---- BRDF ----

fn calculate_brdf(ray: Ray, material: Material) -> BRDFOutput {
    let lambertian_ray = normalize(hugues_moller(hit_record.n) * cosine_sample()); // Lambertian
    let reflection_ray = normalize(reflect(ray.dir, hit_record.n)); // Reflection
    let new_ray_dir = mix(reflection_ray, lambertian_ray, material.roughness);

    var albedo = material.albedo;
    var metallic = material.metallic;
    var roughness = material.roughness;
    if material.albedo_texture != U32_MAX {
        albedo *= sample_texture(material.albedo_texture, hit_record.uv.x, hit_record.uv.y);
    }
    if material.metallic_roughness_texture != U32_MAX {
        let mr = sample_texture(material.metallic_roughness_texture, hit_record.uv.x, hit_record.uv.y);
    }
    
    
    // BRDF Vectors
    let N = hit_record.n; // Surface Normal
    let V = -ray.dir; // View Vector (Outgoing Light)
    let L = new_ray_dir; // Incoming Light
    let R = reflect(-L, N); // reflection vector

    // Dot Products
    let NdotL = dot(N, L);
    let NdotV = max(dot(N, V), 0.0001);

    // Create Lighting Input
    var lighting_input: lighting::LightingInput;
    lighting_input.layers[lighting::LAYER_BASE].NdotV = NdotV;
    lighting_input.layers[lighting::LAYER_BASE].N = N;
    lighting_input.layers[lighting::LAYER_BASE].R = R;
    lighting_input.layers[lighting::LAYER_BASE].perceptual_roughness = roughness;
    lighting_input.layers[lighting::LAYER_BASE].roughness = lighting::perceptualRoughnessToRoughness(roughness);
    lighting_input.P = hit_record.p;
    lighting_input.V = V;
    lighting_input.diffuse_color = albedo;
    lighting_input.F0_ = pbr_functions::calculate_F0(albedo, metallic, material.reflectance);
    lighting_input.F_ab = lighting::F_AB(roughness, NdotV);

    var derived_lighting_input = lighting::derive_lighting_input(N, V, L);

    // let specular = lighting::specular(&lighting_input, &derived_lighting_input, material.reflectance);
    let color = albedo * lighting::Fd_Burley(&lighting_input, &derived_lighting_input);

    // Output
    return BRDFOutput(new_ray_dir, color * PI);
}

// ---- Entry ----

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    // Setup
    let uv = (in.uv - 0.5) * view.viewport.zw / view.viewport.w * vec2<f32>(1.0, -1.0);
    rng_setup(in.uv * view.viewport.zw * (globals.time + 1.0));
    
    let initial_origin = view.world_position;
    let initial_direction = normalize(view.world_from_view * vec4<f32>(uv.x * settings.fov, uv.y * settings.fov, -1.0, 0.0)).xyz;
    
    // Sample
    var pixel_color = vec3<f32>(0.0);
    for (var sample = 0u; sample < settings.samples; sample++) {
        // Setup
        var ray = Ray(initial_origin, initial_direction);
        
        // Tracing
        var ray_color = vec3<f32>(1.0);
        var color = vec3<f32>(0.0);

        for (var bounce = 0u; bounce < settings.bounces; bounce++) {
            hit_record.t = 1000.0;

            let hit = hit_all(ray);
            if hit != U32_MAX {
                let object = objects[hit];
                let material = materials[object.mat];
                let prev_ray_dir = ray.dir;

                // Emissive
                var emissive = material.emissive;
                if material.emissive_texture != U32_MAX {
                    emissive = sample_texture(material.emissive_texture, hit_record.uv.x, hit_record.uv.y);
                }
                
                color += ray_color * emissive;
                if dot(material.albedo, material.albedo) < EPSILON {
                    // Skip Scatter, BRDF and RayColor
                    break;
                }

                // Normal
                if material.normal_map_texture != U32_MAX {
                    hit_record.n *= sample_texture(material.normal_map_texture, hit_record.uv.x, hit_record.uv.y);
                }

                // Scatter
                let brdf = calculate_brdf(ray, material);
                ray.dir = brdf.ray_dir;
                ray.pos = hit_record.p + ray.dir * 0.001;

                ray_color *= brdf.color;
            } else {
                color += ray_color * settings.sky_color;
                break;
            }

            let p = max(ray_color.x, max(ray_color.y, ray_color.z));
            if p < EPSILON {
                break;
            }
        }

        pixel_color += color;
    }

    // Output
    return vec4<f32>(pixel_color / f32(settings.samples), 1.0);
}
