#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_render::{view::View, globals::Globals}
#import bevy_render::maths::{PI, HALF_PI}
#import bevy_pbr::lighting;
#import bevy_pbr::pbr_functions;

const EPSILON: f32 = 4.88e-4;
const INFINITY: f32 = 10000000.0; // 10^8 
const U32_MAX: u32 = 4294967295; // 2**32-1

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<uniform> globals: Globals;
@group(0) @binding(2) var<uniform> settings: Settings;

@group(1) @binding(0) var<storage> objects: array<Object>;
@group(1) @binding(1) var<storage> emissives: array<u32>;

@group(2) @binding(0) var<storage> materials: array<Material>;
@group(2) @binding(1) var<storage> meshes: array<Mesh>;
@group(2) @binding(2) var<storage> indices: array<u32>;
@group(2) @binding(3) var<storage> vertices: array<Vertex>;

@group(3) @binding(0) var<storage> textures: array<Texture>;
@group(3) @binding(1) var<storage> texture_data: array<vec3<f32>>;

// ---- Binding Data ----

struct Settings {
    bounces: u32,
    samples: u32,
    fov: f32,
    sky_color: vec3<f32>,
}

struct Object {
    local_to_world: mat4x4<f32>,
    world_to_local: mat4x4<f32>,
    
    mat: u32,
    mesh: u32,
}

struct Material {
    albedo: vec3<f32>,
    emissive: vec3<f32>,
    roughness: f32,
    metallic: f32,
    reflectance: f32,
}

struct Mesh {
    aabb_min: vec3<f32>,
    aabb_max: vec3<f32>,
    
    ihead: u32,
    vhead: u32,
    tri_count: u32,
}

struct Vertex {
    position: vec3<f32>,
    normal: vec3<f32>,
    uv: vec2<f32>,
}

struct Texture {
    width: u32,
    height: u32,
    offset: u32,
}

// --- Runtime Data ----

struct Ray {
    pos: vec3<f32>,
    dir: vec3<f32>,
}

struct HitRecord {
    t: f32,
    p: vec3<f32>,
    n: vec3<f32>,
    uv: vec2<f32>,
}

struct BRDFOutput {
    ray_dir: vec3<f32>,
    color: vec3<f32>,
}

var<private> hit_record: HitRecord;
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

// ---- BRDF ----

fn calculate_brdf(ray: Ray, material: Material) -> BRDFOutput {
    let lambertian_ray = normalize(hugues_moller(hit_record.n) * cosine_sample()); // Lambertian
    let reflection_ray = normalize(ray.dir - 2.0 * dot(ray.dir, hit_record.n) * hit_record.n); // Reflection
    let new_ray_dir = mix(reflection_ray, lambertian_ray, material.roughness);
    
    // BRDF Vectors
    let N = hit_record.n; // Surface Normal
    let V = -ray.dir; // View Vector (Outgoing Light)
    let L = new_ray_dir; // Incoming Light
    // let H = V + (L - V) * 0.5; // Half-way Vector (between _v and _l)
    let R = normalize(-L - 2.0 * dot(-L, N) * N); // reflection vector
    // let T = normalize(cross(N, V)); // normal tangent
    // let v = normalize(V - (N * V)*N); // _v projected onto normal
    // let l = normalize(L - (V * L)*N); // _l projected onto normal

    // Dot Products
    let NdotL = dot(N, L);
    let NdotV = dot(N, V);
    // let NdotH = dot(N, H);

    // let LdotH = dot(L, H);

    // Create Lighting Input
    var lighting_input: lighting::LightingInput;
    lighting_input.layers[lighting::LAYER_BASE].NdotV = NdotV;
    lighting_input.layers[lighting::LAYER_BASE].N = N;
    lighting_input.layers[lighting::LAYER_BASE].R = R;
    lighting_input.layers[lighting::LAYER_BASE].perceptual_roughness = material.roughness;
    lighting_input.layers[lighting::LAYER_BASE].roughness = lighting::perceptualRoughnessToRoughness(material.roughness);
    lighting_input.P = hit_record.p;
    lighting_input.V = V;
    lighting_input.diffuse_color = material.albedo;
    lighting_input.F0_ = pbr_functions::calculate_F0(material.albedo, material.metallic, material.reflectance);
    lighting_input.F_ab = lighting::F_AB(material.roughness, NdotV);

    var derived_lighting_input = lighting::derive_lighting_input(N, V, L);

    let specular = lighting::specular(&lighting_input, &derived_lighting_input, material.reflectance);
    let color = material.albedo * lighting::Fd_Burley(&lighting_input, &derived_lighting_input) + specular;

    // Output
    return BRDFOutput(new_ray_dir, color);
}

// ---- Entry ----

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    // Setup
    let uv = (in.uv - 0.5) * view.viewport.zw / view.viewport.w * vec2<f32>(1.0, -1.0);
    rng_setup(in.uv * view.viewport.zw * (globals.time + 1.0));
    var ray = Ray();
    
    // Sample
    var pixel_color = vec3<f32>(0.0);
    for (var sample = 0u; sample < settings.samples; sample++) {
        // Setup
        // Orthographic
        // ray.pos = view.world_position + (view.world_from_view * vec4<f32>(uv * 4.0, 0.0, 0.0)).xyz;
        // ray.dir = normalize(view.world_from_view * vec4<f32>(0.0, 0.0, -1.0, 0.0)).xyz;

        // Perspective
        ray.pos = view.world_position;
        ray.dir = normalize(view.world_from_view * vec4<f32>(uv.x * settings.fov, uv.y * settings.fov, -1.0, 0.0)).xyz;
        
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
                color += ray_color * material.emissive;
                if dot(material.albedo, material.albedo) < EPSILON {
                    // Skip Scatter, BRDF and RayColor
                    break;
                }

                // Scatter
                let brdf = calculate_brdf(ray, material);
                ray.dir = brdf.ray_dir;
                ray.pos = hit_record.p + ray.dir * 0.000001;

                ray_color *= brdf.color;
            } else {
                color += ray_color * settings.sky_color;
                break;
            }

            let p = max(ray_color.x, max(ray_color.y, ray_color.z));
            if p < EPSILON {
                break;
            }
            ray_color *= 1.0 / (1.0 + p);
        }

        pixel_color += color;
    }

    // Output
    return vec4<f32>(pixel_color / f32(settings.samples), 1.0);
}

// ---- Hit Checks ----

fn hit_all(ray: Ray) -> u32 {
    var hit = U32_MAX;
    for (var o = 0u; o < arrayLength(&objects); o++) {
        if hit_mesh(o, 0.0001, ray) {
            hit = o;
        }
    }

    return hit;
}

fn hit_mesh(object_index: u32, t_min: f32, _ray: Ray) -> bool {
    let object = &objects[object_index];
    let mesh = &meshes[(*object).mesh];
    var hit = false;

    // Ray World to Local space
    var ray = _ray;
    ray.pos = ((*object).world_to_local * vec4<f32>(ray.pos, 1.0)).xyz;
    ray.dir = ((*object).world_to_local * vec4<f32>(ray.dir, 0.0)).xyz;

    // Ray-Box Test
    let t_aabb = hit_box((*mesh).aabb_min, (*mesh).aabb_max, t_min, ray);
    if t_aabb < t_min {
        return false;
    }
    
    // Ray-Triangle tests
    for (var i = 0u; i < (*mesh).tri_count; i++) {
        let i = i * 3;

        let ai = indices[(*mesh).ihead + i];
        let bi = indices[(*mesh).ihead + i + 1];
        let ci = indices[(*mesh).ihead + i + 2];

        var va = vertices[(*mesh).vhead + ai];
        var vb = vertices[(*mesh).vhead + bi];
        var vc = vertices[(*mesh).vhead + ci];
        
        // Möller–Trumbore
        let edge_ab = vb.position - va.position;
        let edge_ac = vc.position - va.position;
        let n = cross(edge_ab, edge_ac);
        let ao = ray.pos - va.position;
        let dao = cross(ao, ray.dir);

        let det = dot(-ray.dir, n);
        let inv_det = 1.0 / det;

        let t = dot(ao, n) * inv_det;
        let u = dot(edge_ac, dao) * inv_det;
        let v = dot(-edge_ab, dao) * inv_det;
        let w = 1.0 - u - v;

        if det < EPSILON || t < t_min || t > hit_record.t || u < 0.0 || v < 0.0 || w < 0.0 {
            continue;
        }

        let _p = ray.pos + ray.dir * t;
        let _n = va.normal * w + vb.normal * u + vc.normal * v;

        hit_record.t = t;
        hit_record.p = ((*object).local_to_world * vec4<f32>(_p, 1.0)).xyz;
        hit_record.n = normalize( ((*object).local_to_world * vec4<f32>(_n, 0.0)).xyz );
        hit_record.uv = va.uv * w + vb.uv * v + vc.uv * u;
        hit = true;
    }

    return hit;
}

fn hit_box(min: vec3<f32>, max: vec3<f32>, _tmin: f32, ray: Ray) -> f32 {
    let inv_dir = 1.0 / ray.dir;
    var tmin = (min - ray.pos) * inv_dir;
    var tmax = (max - ray.pos) * inv_dir;
    
    let t1 = min(tmin, tmax);
    let t2 = max(tmin, tmax);
    let dst_near = max(max(t1.x, t1.y), t1.z);
    let dst_far = min(min(t2.x, t2.y), t2.z);

    let hit = dst_far >= dst_near && dst_far > 0;
    if hit {
        if dst_near > 0.0 {
            return dst_near;
        } else {
            return INFINITY;
        }
    } else {
        return 0.0;
    }
}
