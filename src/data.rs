use bevy::{
    asset::UntypedAssetId,
    color::LinearRgba,
    ecs::{component::Component, system::Resource},
    math::{Mat4, Vec2, Vec3},
    prelude::{Image, Mesh as BevyMesh},
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
    pub albedo_texture: u32,
    pub emissive: Vec3,
    pub emissive_texture: u32,
    pub roughness: f32,
    pub metallic: f32,
    pub metallic_roughness_texture: u32,
    pub reflectance: f32,
    pub normal_map_texture: u32,
}

#[derive(Component, Default, Clone, Copy, ShaderType)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub offset: u32,
    pub format: u32,
}

#[derive(Default)]
pub struct TextureData {
    pub data: Vec<f32>,
}

#[derive(Component, Default, Clone, Copy, ShaderType)]
pub struct GpuMesh {
    pub aabb_min: Vec3,
    pub aabb_max: Vec3,

    pub ihead: u32,
    pub vhead: u32,
    pub tri_count: u32,
}

#[derive(Default, Clone, Copy, ShaderType)]
pub struct GpuVertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
}

pub struct CpuMesh {
    pub aabb_min: Vec3,
    pub aabb_max: Vec3,

    pub indices: Vec<u32>,
    pub vertices: Vec<GpuVertex>,
}

#[derive(Resource)]
pub struct RayTraceMeta {
    pub objects: StorageBuffer<Vec<Object>>,
    pub emissives: StorageBuffer<Vec<u32>>,

    pub meshes: StorageBuffer<Vec<GpuMesh>>,
    pub indices: StorageBuffer<Vec<u32>>,
    pub vertices: StorageBuffer<Vec<GpuVertex>>,

    pub handle_to_material: HashMap<UntypedAssetId, usize>,
    pub handle_to_texture: HashMap<UntypedAssetId, usize>,
    pub materials: StorageBuffer<Vec<Material>>,
    pub textures: StorageBuffer<Vec<Texture>>,
    pub texture_data: StorageBuffer<Vec<f32>>,
}

impl TextureData {
    pub fn append_texture(&mut self, image: &Image) -> Texture {
        use bevy::render::render_resource::TextureFormat as WgpuTextureFormat;
        const DIV_255: f32 = 1.0 / 255.0;

        let offset = self.data.len() as u32;
        let format = match image.texture_descriptor.format {
            WgpuTextureFormat::Rgba8UnormSrgb => {
                image
                    .data
                    .chunks(1)
                    .for_each(|c| self.data.push(c[0] as f32 * DIV_255));
                4
            }
            WgpuTextureFormat::Rgba16Float => {
                image.data.chunks(2).for_each(|c| {
                    self.data
                        .push(f16::from_le_bytes([c[0], c[1]]) as f32 * DIV_255)
                });
                4
            }
            WgpuTextureFormat::Rgb9e5Ufloat => {
                image.data.chunks(4).for_each(|d| {
                    let e = (d[3] << 2) & 0b011111;
                    let r = [e | (d[0] >> 7), (d[0] << 1) | ((d[1] & 0b1) >> 7)];
                    let g = [e | ((d[1] & 0b01) >> 6), (d[1] << 2) | ((d[2] & 0b11) >> 6)];
                    let b = [
                        e | ((d[2] & 0b001) >> 5),
                        (d[2] << 3) | ((d[3] & 0b111) >> 5),
                    ];

                    self.data.push(f16::from_le_bytes(r) as f32);
                    self.data.push(f16::from_le_bytes(g) as f32);
                    self.data.push(f16::from_le_bytes(b) as f32);
                });
                3
            }
            WgpuTextureFormat::R8Unorm => {
                image.data.chunks(1).for_each(|r| {
                    self.data.push(r[0] as f32 * DIV_255);
                });
                1
            }
            WgpuTextureFormat::Rg8Unorm => {
                image.data.chunks(1).for_each(|r| {
                    self.data.push(r[0] as f32 * DIV_255);
                });
                2
            }
            f => {
                panic!("Texture format {:?} is not supported.", f);
            }
        };

        Texture {
            width: image.width(),
            height: image.height(),
            offset,
            format,
        }
    }
}
