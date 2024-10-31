use crate::data::{self, CpuMesh, GpuMesh, RayTraceMeta, TextureData};
use bevy::{
    prelude::*,
    render::{
        mesh::VertexAttributeValues,
        renderer::{RenderDevice, RenderQueue},
        Extract,
    },
    utils::HashMap,
};

#[derive(Resource)]
pub struct ProcessedMeshes {
    pub meshes: Vec<CpuMesh>,
    pub asset_to_index: HashMap<AssetId<Mesh>, usize>,
    pub changed: bool,
}

pub fn extract_meshes(
    mesh_assets: Extract<Res<Assets<Mesh>>>,
    mut asset_events: Extract<EventReader<AssetEvent<Mesh>>>,
    mut processed_meshes: ResMut<ProcessedMeshes>,
) {
    let mut remove = Vec::new();
    let mut extract = Vec::new();

    for event in asset_events.read() {
        match event {
            AssetEvent::Added { id }
            | AssetEvent::Modified { id }
            | AssetEvent::LoadedWithDependencies { id } => {
                extract.push(*id);
            }
            AssetEvent::Removed { id } | AssetEvent::Unused { id } => {
                remove.push(*id);
            }
        }

        processed_meshes.changed = true;
    }

    for id in remove {
        let index = processed_meshes.asset_to_index.remove(&id).unwrap();
        processed_meshes.meshes.remove(index);

        for ati in processed_meshes.asset_to_index.values_mut() {
            if *ati < index {
                continue;
            }

            *ati -= 1;
        }
    }

    for id in extract {
        let mesh = mesh_assets.get(id).unwrap();
        let mut cpu = CpuMesh {
            aabb_min: Vec3::INFINITY,
            aabb_max: Vec3::NEG_INFINITY,
            indices: Vec::new(),
            vertices: Vec::new(),
        };

        let (
            Some(VertexAttributeValues::Float32x3(positions)),
            Some(VertexAttributeValues::Float32x3(normals)),
            Some(VertexAttributeValues::Float32x2(uvs)),
        ) = (
            mesh.attribute(Mesh::ATTRIBUTE_POSITION),
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL),
            mesh.attribute(Mesh::ATTRIBUTE_UV_0),
        )
        else {
            continue;
        };

        for ((position, normal), uv) in positions.iter().zip(normals).zip(uvs) {
            let position = Vec3::from_array(*position);
            cpu.aabb_min = cpu.aabb_min.min(position);
            cpu.aabb_max = cpu.aabb_max.max(position);

            cpu.vertices.push(data::GpuVertex {
                position,
                normal: Vec3::from_array(*normal),
                uv: Vec2::from_array(*uv),
            });
        }

        cpu.indices = match mesh.indices().unwrap() {
            bevy::render::mesh::Indices::U16(vec) => {
                vec.iter().cloned().map(|v| v as u32).collect()
            }
            bevy::render::mesh::Indices::U32(vec) => vec.clone(),
        };

        let index = processed_meshes.meshes.len();
        processed_meshes.meshes.push(cpu);
        processed_meshes.asset_to_index.insert(id, index);
    }
}

pub fn prepare_meshes(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut processed_meshes: ResMut<ProcessedMeshes>,
    mut raytrace_meta: ResMut<RayTraceMeta>,
) {
    if !processed_meshes.changed {
        return;
    }
    processed_meshes.changed = false;

    let mut meshes = Vec::with_capacity(processed_meshes.meshes.len());
    let mut indices = Vec::new();
    let mut vertices = Vec::new();

    for mesh in &processed_meshes.meshes {
        let gpu_mesh = GpuMesh {
            aabb_min: mesh.aabb_min,
            aabb_max: mesh.aabb_max,
            ihead: indices.len() as u32,
            vhead: vertices.len() as u32,
            tri_count: (mesh.indices.len() / 3) as u32,
        };

        indices.extend_from_slice(&mesh.indices);
        vertices.extend_from_slice(&mesh.vertices);
        meshes.push(gpu_mesh);
    }

    // Write
    *(raytrace_meta.meshes.get_mut()) = meshes;
    *(raytrace_meta.indices.get_mut()) = indices;
    *(raytrace_meta.vertices.get_mut()) = vertices;

    raytrace_meta
        .meshes
        .write_buffer(&render_device, &render_queue);
    raytrace_meta
        .indices
        .write_buffer(&render_device, &render_queue);
    raytrace_meta
        .vertices
        .write_buffer(&render_device, &render_queue);

    debug!("Wrote meshes to gpu buffer");
}

// pub fn extract_meshes(
//     render_device: Extract<Res<RenderDevice>>,
//     render_queue: Extract<Res<RenderQueue>>,
//     mesh_assets: Extract<Res<Assets<Mesh>>>,
//     mut raytrace_meta: ResMut<RayTraceMeta>,
//     mut mesh_count: Local<usize>,
// ) {
//     if mesh_assets.len() == *mesh_count {
//         return;
//     }

//     raytrace_meta.handle_to_mesh.clear();
//     *mesh_count = mesh_assets.len();

//     let mut meshes = Vec::new();
//     let mut mesh_data = MeshData::default();

//     for (id, mesh) in mesh_assets.iter() {
//         raytrace_meta
//             .handle_to_mesh
//             .insert(id.untyped(), meshes.len());
//         meshes.push(mesh_data.append_mesh(mesh));
//     }

//     // Mesh Meta
//     *(raytrace_meta.meshes.get_mut()) = meshes;
//     *(raytrace_meta.indices.get_mut()) = mesh_data.indices;
//     *(raytrace_meta.vertices.get_mut()) = mesh_data.vertices;

//     raytrace_meta
//         .meshes
//         .write_buffer(&render_device, &render_queue);
//     raytrace_meta
//         .indices
//         .write_buffer(&render_device, &render_queue);
//     raytrace_meta
//         .vertices
//         .write_buffer(&render_device, &render_queue);

//     debug!("Wrote meshes to gpu buffer");
// }

pub fn extract_materials(
    render_device: Extract<Res<RenderDevice>>,
    render_queue: Extract<Res<RenderQueue>>,
    material_assets: Extract<Res<Assets<StandardMaterial>>>,
    mut raytrace_meta: ResMut<RayTraceMeta>,
    mut material_count: Local<usize>,
) {
    if material_assets.len() == *material_count {
        return;
    }

    raytrace_meta.handle_to_material.clear();
    *material_count = material_assets.len();

    let mut materials = Vec::new();

    for (id, material) in material_assets.iter() {
        raytrace_meta
            .handle_to_material
            .insert(id.untyped(), materials.len());

        let albedo_texture = material
            .base_color_texture
            .as_ref()
            .map(|handle| raytrace_meta.handle_to_texture.get(&handle.id().untyped()))
            .flatten();
        let emissive_texture = material
            .emissive_texture
            .as_ref()
            .map(|handle| raytrace_meta.handle_to_texture.get(&handle.id().untyped()))
            .flatten();
        let metallic_roughness_texture = material
            .metallic_roughness_texture
            .as_ref()
            .map(|handle| raytrace_meta.handle_to_texture.get(&handle.id().untyped()))
            .flatten();
        let normal_map_texture = material
            .normal_map_texture
            .as_ref()
            .map(|handle| raytrace_meta.handle_to_texture.get(&handle.id().untyped()))
            .flatten();

        materials.push(data::Material {
            albedo: material.base_color.to_linear().to_vec3(),
            albedo_texture: albedo_texture.map(|v| *v as u32).unwrap_or(u32::MAX),
            emissive: material.emissive.to_vec3(),
            emissive_texture: emissive_texture.map(|v| *v as u32).unwrap_or(u32::MAX),
            roughness: material.perceptual_roughness,
            metallic: material.metallic,
            metallic_roughness_texture: metallic_roughness_texture
                .map(|v| *v as u32)
                .unwrap_or(u32::MAX),
            reflectance: material.reflectance,
            normal_map_texture: normal_map_texture.map(|v| *v as u32).unwrap_or(u32::MAX),
        });
    }

    // Material Meta
    *(raytrace_meta.materials.get_mut()) = materials;

    raytrace_meta
        .materials
        .write_buffer(&render_device, &render_queue);

    debug!("Wrote materials to gpu buffer");
}

pub fn extract_textures(
    render_device: Extract<Res<RenderDevice>>,
    render_queue: Extract<Res<RenderQueue>>,
    image_assets: Extract<Res<Assets<Image>>>,
    mut raytrace_meta: ResMut<RayTraceMeta>,
    mut image_count: Local<usize>,
) {
    if image_assets.len() == *image_count {
        return;
    }

    raytrace_meta.handle_to_texture.clear();
    *image_count = image_assets.len();

    let mut textures = Vec::new();
    let mut texture_data = TextureData::default();

    for (id, image) in image_assets.iter() {
        raytrace_meta
            .handle_to_texture
            .insert(id.untyped(), textures.len());

        textures.push(texture_data.append_texture(image));
    }

    // Texture Meta
    *(raytrace_meta.textures.get_mut()) = textures;
    *(raytrace_meta.texture_data.get_mut()) = texture_data.data;

    raytrace_meta
        .textures
        .write_buffer(&render_device, &render_queue);
    raytrace_meta
        .texture_data
        .write_buffer(&render_device, &render_queue);

    debug!("Wrote textures to gpu buffer");
}

pub fn extract_visible(
    render_device: Extract<Res<RenderDevice>>,
    render_queue: Extract<Res<RenderQueue>>,

    material_assets: Extract<Res<Assets<StandardMaterial>>>,
    query: Extract<Query<(&GlobalTransform, &Mesh3d, &MeshMaterial3d<StandardMaterial>)>>,
    processed_meshes: Res<ProcessedMeshes>,
    mut raytrace_meta: ResMut<RayTraceMeta>,
) {
    let mut objects = Vec::new();
    let mut emissives = Vec::new();

    for (transform, mesh_handle, mat_handle) in query.iter() {
        if let Some(mat) = material_assets.get(mat_handle) {
            if mat.emissive.red > 0.0 || mat.emissive.green > 0.0 || mat.emissive.blue > 0.0 {
                emissives.push(objects.len() as u32);
            }
        }

        let Some(&mesh) = processed_meshes.asset_to_index.get(&mesh_handle.id()) else {
            continue;
        };
        let Some(&mat) = raytrace_meta
            .handle_to_material
            .get(&mat_handle.id().untyped())
        else {
            continue;
        };

        let local_to_world = transform.compute_matrix();
        objects.push(data::Object {
            world_to_local: local_to_world.inverse(),
            local_to_world,

            mat: mat as u32,
            mesh: mesh as u32,
        });
    }

    // Query Meta
    *(raytrace_meta.objects.get_mut()) = objects;
    *(raytrace_meta.emissives.get_mut()) = emissives;

    raytrace_meta
        .objects
        .write_buffer(&render_device, &render_queue);
    raytrace_meta
        .emissives
        .write_buffer(&render_device, &render_queue);
}
