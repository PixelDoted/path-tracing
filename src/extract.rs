use crate::data::{self, MeshData, RayTraceMeta, TextureData};
use bevy::{
    prelude::*,
    render::{
        renderer::{RenderDevice, RenderQueue},
        Extract,
    },
};

pub fn extract_meshes(
    render_device: Extract<Res<RenderDevice>>,
    render_queue: Extract<Res<RenderQueue>>,
    mesh_assets: Extract<Res<Assets<Mesh>>>,
    mut raytrace_meta: ResMut<RayTraceMeta>,
    mut mesh_count: Local<usize>,
) {
    if mesh_assets.len() == *mesh_count {
        return;
    }

    raytrace_meta.handle_to_mesh.clear();
    *mesh_count = mesh_assets.len();

    let mut meshes = Vec::new();
    let mut mesh_data = MeshData::default();

    for (id, mesh) in mesh_assets.iter() {
        raytrace_meta
            .handle_to_mesh
            .insert(id.untyped(), meshes.len());
        meshes.push(mesh_data.append_mesh(mesh));
    }

    // Mesh Meta
    *(raytrace_meta.meshes.get_mut()) = meshes;
    *(raytrace_meta.indices.get_mut()) = mesh_data.indices;
    *(raytrace_meta.vertices.get_mut()) = mesh_data.vertices;

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

        let local_to_world = transform.compute_matrix();
        objects.push(data::Object {
            world_to_local: local_to_world.inverse(),
            local_to_world,

            mat: *raytrace_meta
                .handle_to_material
                .get(&mat_handle.id().untyped())
                .unwrap() as u32,
            mesh: *raytrace_meta
                .handle_to_mesh
                .get(&mesh_handle.id().untyped())
                .unwrap() as u32,
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
