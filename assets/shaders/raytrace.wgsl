#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_render::{view::View, globals::Globals}

const EPSILON: f32 = 4.88e-4;
const PI: f32 = 3.141592653589793;
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
    let a = abs(hit_record.n);
    var t = vec3<f32>(0);
    if a.x <= a.y && a.x <= a.z {
        t = vec3<f32>(0, -hit_record.n.z, hit_record.n.y);
    } else if a.y <= a.x && a.y <= a.z {
        t = vec3<f32>(-hit_record.n.z, 0, hit_record.n.x);
    } else {
        t = vec3<f32>(-hit_record.n.y, hit_record.n.x, 0);
    }
    t = normalize(t);
                    
    let b = normalize(cross(hit_record.n, t));
    return mat3x3<f32>(t, b, hit_record.n);
}

// ---- BRDF ----


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
        // ray.pos = view.world_position + (view.view * vec4<f32>(uv * 4.0, 0.0, 0.0)).xyz;
        // ray.dir = normalize(view.view * vec4<f32>(0.0, 0.0, -1.0, 0.0)).xyz;

        // Perspective
        ray.pos = view.world_position;
        ray.dir = normalize(view.world_from_view * vec4<f32>(uv.x * settings.fov, uv.y * settings.fov, -0.95, 0.0)).xyz;
        
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
                ray.dir = normalize(hugues_moller(hit_record.n) * cosine_sample()); // Lambertian
                // ray.dir = normalize(ray.dir - 2.0 * dot(ray.dir, hit_record.n) * hit_record.n); // Reflection
                ray.pos = hit_record.p + ray.dir * 0.001;

                // BRDF Vectors
                let _n = hit_record.n;
                let _v = -prev_ray_dir;
                let _l = ray.dir;
                let _h = _v + (_l - _v) * 0.5;
                let _r = normalize(-_l - 2.0 * dot(-_l, _n) * _n);
                // let _t = vec3<f32>(0.0); // TODO
                // let v = _n * dot(_v, _n);
                // let l = _n * dot(_l, _n);

                // Color
                let diffuse = material.albedo * dot(_n, _l);
                // let specular = vec3<f32>(1.0) * dot(_v, _r);

                ray_color *= diffuse;// + specular;
            } else {
                color += ray_color * settings.sky_color;
                break;
            }

            let p = max(ray_color.x, max(ray_color.y, ray_color.z));
            if p < EPSILON {
                break;
            }
            ray_color *= 1.0 / p;

            // indirect lighting
            // let result = pick_emissive(ray);
            // if result != U32_MAX {
            //     let object = objects[result];
            //     let material = materials[object.mat];
            //     color += ray_color * material.emissive;
            // } 
        }

        pixel_color += color;
    }

    // Output
    return vec4<f32>(pixel_color / f32(settings.samples), 1.0);
}

// ---- Pick ----
fn pick_emissive(_ray: Ray) -> u32 {
    let rng = rand();
    let emissive_index = u32(rng.x * f32(arrayLength(&emissives)));
    let emissive = &emissives[emissive_index];
    let object = &objects[*emissive];
    let mesh = &meshes[(*object).mesh];
    let tri = u32(rng.y * f32((*mesh).tri_count) * 3);

    // Decode Triangle
    let ai = indices[(*mesh).ihead + tri];
    let bi = indices[(*mesh).ihead + tri + 1];
    let ci = indices[(*mesh).ihead + tri + 2];

    let va = vertices[(*mesh).vhead + ai];
    let vb = vertices[(*mesh).vhead + bi];
    let vc = vertices[(*mesh).vhead + ci];

    // Get Point in triangle
    let trng = normalize(rand());
    let p = ((*object).local_to_world * vec4<f32>(va.position*trng.x + vb.position*trng.y + vc.position*trng.z, 1.0)).xyz;
    var ray = _ray;
    ray.dir = p - ray.pos;

    // Out
    if hit_all(ray) == *emissive {
        return *emissive;
    }

    return U32_MAX;
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
        let _n = normalize(va.normal * w + vb.normal * u + vc.normal * v);

        hit_record.t = t;
        hit_record.p = ((*object).local_to_world * vec4<f32>(_p, 1.0)).xyz;
        hit_record.n = ((*object).local_to_world * vec4<f32>(_n, 0.0)).xyz;
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
