use bevy::{
    asset::UntypedAssetId,
    color::LinearRgba,
    ecs::{component::Component, system::Resource},
    math::{Mat4, Vec2, Vec3},
    prelude::Mesh as BevyMesh,
    render::{
        extract_component::ExtractComponent,
        mesh::VertexAttributeValues,
        render_resource::{ShaderType, StorageBuffer},
    },
    utils::HashMap,
};

#[derive(Component, Default, Clone, Copy, ExtractComponent, ShaderType)]
pub struct RayTraceSettings {
    pub bounces: u32,
    pub samples: u32,
    pub fov: f32,
    pub sky_color: LinearRgba,
}

// ---- Shader ----
#[derive(Component, Default, Clone, Copy, ShaderType)]
pub struct Object {
    pub local_to_world: Mat4,
    pub world_to_local: Mat4,

    pub mat: u32,
    pub mesh: u32,
}

#[derive(Component, Default, Clone, Copy, ShaderType)]
pub struct Material {
    pub albedo: Vec3,
    pub emissive: Vec3,
    pub roughness: f32,
    pub metallic: f32,
    pub reflectance: f32,
}

#[derive(Component, Default, Clone, Copy, ShaderType)]
pub struct Texture {
    pub start: u32,
    pub length: u32,
    pub format: u32,
}

#[derive(Default)]
pub struct TextureData {
    pub data: Vec<u32>,
}

#[derive(Component, Default, Clone, Copy, ShaderType)]
pub struct Mesh {
    pub aabb_min: Vec3,
    pub aabb_max: Vec3,

    pub ihead: u32,
    pub vhead: u32,
    pub tri_count: u32,
}

#[derive(Default, Clone, Copy, ShaderType)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
}

#[derive(Default)]
pub struct MeshData {
    pub indices: Vec<u32>,
    pub vertices: Vec<Vertex>,
}

#[derive(Resource)]
pub struct RayTraceMeta {
    pub objects: StorageBuffer<Vec<Object>>,
    pub emissives: StorageBuffer<Vec<u32>>,

    pub handle_to_material: HashMap<UntypedAssetId, usize>,
    pub materials: StorageBuffer<Vec<Material>>,
    pub textures: StorageBuffer<Vec<Texture>>,
    pub texture_data: StorageBuffer<Vec<u32>>,

    pub handle_to_mesh: HashMap<UntypedAssetId, usize>,
    pub meshes: StorageBuffer<Vec<Mesh>>,
    pub indices: StorageBuffer<Vec<u32>>,
    pub vertices: StorageBuffer<Vec<Vertex>>,
}

impl TextureData {
    pub fn append_texture(&mut self) {
        todo!();
    }
}

impl MeshData {
    pub fn append_mesh(&mut self, mesh: &BevyMesh) -> Mesh {
        let indices = mesh.indices().expect("Mesh has no indices");
        let positions = mesh
            .attribute(BevyMesh::ATTRIBUTE_POSITION)
            .expect("Mesh has no vertices");
        let normals = mesh
            .attribute(BevyMesh::ATTRIBUTE_NORMAL)
            .expect("Mesh has no normals");
        let uvs = mesh
            .attribute(BevyMesh::ATTRIBUTE_UV_0)
            .expect("Mesh has no uvs");

        let mut mesh = Mesh {
            aabb_min: Vec3::INFINITY,
            aabb_max: Vec3::NEG_INFINITY,
            ihead: self.indices.len() as u32,
            vhead: self.vertices.len() as u32,
            tri_count: (indices.len() / 3) as u32,
        };

        for i in indices.iter() {
            self.indices.push(i as u32);
        }

        let mut i = 0;
        while let Some(position) = match positions {
            VertexAttributeValues::Float32x3(values) => values.get(i),
            _ => None,
        } {
            let normal = match normals {
                VertexAttributeValues::Float32x3(values) => values[i],
                _ => panic!("Normal format has to be `Float32x3`"),
            };
            let uv = match uvs {
                VertexAttributeValues::Float32x2(values) => values[i],
                _ => panic!("UV format has to be `Float32x2`"),
            };

            mesh.aabb_min.x = mesh.aabb_min.x.min(position[0]);
            mesh.aabb_min.y = mesh.aabb_min.y.min(position[1]);
            mesh.aabb_min.z = mesh.aabb_min.z.min(position[2]);
            mesh.aabb_max.x = mesh.aabb_max.x.max(position[0]);
            mesh.aabb_max.y = mesh.aabb_max.y.max(position[1]);
            mesh.aabb_max.z = mesh.aabb_max.z.max(position[2]);

            self.vertices.push(Vertex {
                position: Vec3::from_array(*position),
                normal: Vec3::from_array(normal),
                uv: Vec2::from_array(uv),
            });
            i += 1;
        }

        mesh
    }
}
