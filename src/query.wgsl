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
    start_index: u32,
    start_vertex: u32,
    end_index: u32,
    end_vertex: u32,
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
