#define_import_path path_tracing::query

#import bevy_render::maths::{PI, HALF_PI}

#import path_tracing::math::{EPSILON, U32_MAX, INFINITY, T_MIN}

// Bindings
@group(1) @binding(0) var<storage> objects: array<Object>;
@group(1) @binding(1) var<storage> emissives: array<u32>;

@group(2) @binding(0) var<storage> meshes: array<Mesh>;
@group(2) @binding(1) var<storage> indices: array<u32>;
@group(2) @binding(2) var<storage> vertices: array<Vertex>;

// Mesh Types
struct Object {
    local_to_world: mat4x4<f32>,
    world_to_local: mat4x4<f32>,
    
    mat: u32,
    mesh: u32,
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

// Ray Types
struct Ray {
    pos: vec3<f32>,
    dir: vec3<f32>,
}

struct HitRecord {
    t: f32,
    p: vec3<f32>,
    n: vec3<f32>,
    uv: vec2<f32>,
    area: f32,
}

var<private> hit_record: HitRecord;

// Functions
fn hit_all(ray: Ray) -> u32 {
    var hit = U32_MAX;
    for (var o = 0u; o < arrayLength(&objects); o++) {
        if hit_mesh(o, T_MIN, ray) {
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
        let _uv = va.uv * w + vb.uv * u + vc.uv * v;

        let area = length(cross(edge_ab, edge_ac)) * 0.5;

        hit_record.t = t;
        hit_record.p = ((*object).local_to_world * vec4<f32>(_p, 1.0)).xyz;
        hit_record.n = normalize( ((*object).local_to_world * vec4<f32>(_n, 0.0)).xyz );
        hit_record.uv = _uv;
        hit_record.area = area;
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
