use bevy::{
    asset::UntypedAssetId,
    core_pipeline::{
        core_3d::graph::{Core3d, Node3d},
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    },
    prelude::*,
    render::{
        extract_component::{ComponentUniforms, ExtractComponentPlugin, UniformComponentPlugin},
        globals::{GlobalsBuffer, GlobalsUniform},
        render_graph::{RenderGraphApp, RenderLabel, ViewNode, ViewNodeRunner},
        render_resource::{
            binding_types::uniform_buffer, BindGroupEntries, BindGroupLayout,
            BindGroupLayoutEntries, BindingType, BufferBindingType, CachedRenderPipelineId,
            ColorTargetState, ColorWrites, FragmentState, MultisampleState, Operations,
            PipelineCache, PrimitiveState, RenderPassColorAttachment, RenderPassDescriptor,
            RenderPipelineDescriptor, ShaderStages, ShaderType, StorageBuffer, TextureFormat,
        },
        renderer::{RenderDevice, RenderQueue},
        view::{ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
        Extract, RenderApp,
    },
    utils::HashMap,
};

use crate::data::{self, MeshData, RayTraceMeta, RayTraceSettings, Texture, TextureData, Vertex};

pub struct RayTracePlugin;

impl Plugin for RayTracePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractComponentPlugin::<RayTraceSettings>::default(),
            UniformComponentPlugin::<RayTraceSettings>::default(),
        ));

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.insert_resource(RayTraceMeta {
            objects: StorageBuffer::default(),
            emissives: StorageBuffer::default(),

            handle_to_material: HashMap::new(),
            materials: StorageBuffer::default(),

            handle_to_texture: HashMap::new(),
            textures: StorageBuffer::default(),
            texture_data: StorageBuffer::default(),

            handle_to_mesh: HashMap::new(),
            meshes: StorageBuffer::default(),
            indices: StorageBuffer::default(),
            vertices: StorageBuffer::default(),
        });

        render_app.add_systems(
            ExtractSchedule,
            (
                (
                    extract_meshes,
                    (extract_textures, extract_materials).chain(),
                ),
                extract_visible,
            )
                .chain(),
        );
        render_app
            .add_render_graph_node::<ViewNodeRunner<RayTraceNode>>(Core3d, RayTraceLabel)
            .add_render_graph_edges(
                Core3d,
                (Node3d::EndMainPass, RayTraceLabel, Node3d::MotionBlur),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<RayTracePipeline>();
    }
}

// Extract
fn extract_meshes(
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

fn extract_materials(
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

fn extract_textures(
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

fn extract_visible(
    render_device: Extract<Res<RenderDevice>>,
    render_queue: Extract<Res<RenderQueue>>,

    material_assets: Extract<Res<Assets<StandardMaterial>>>,
    query: Extract<Query<(&GlobalTransform, &Handle<Mesh>, &Handle<StandardMaterial>)>>,
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

// Shader

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct RayTraceLabel;

#[derive(Default)]
struct RayTraceNode;

impl ViewNode for RayTraceNode {
    type ViewQuery = (
        &'static ViewUniformOffset,
        &'static ViewTarget,
        &'static RayTraceSettings,
    );

    fn run<'w>(
        &self,
        _graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext<'w>,
        (view_uniform_offset, view_target, _settings): bevy::ecs::query::QueryItem<
            'w,
            Self::ViewQuery,
        >,
        world: &'w World,
    ) -> Result<(), bevy::render::render_graph::NodeRunError> {
        let ray_trace_pipeline = world.resource::<RayTracePipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let Some(pipeline) = pipeline_cache.get_render_pipeline(ray_trace_pipeline.pipeline_id)
        else {
            return Ok(());
        };

        let Some(globals_uniforms) = world.resource::<GlobalsBuffer>().buffer.binding() else {
            return Ok(());
        };

        let settings_uniforms = world.resource::<ComponentUniforms<RayTraceSettings>>();
        let Some(settings_binding) = settings_uniforms.uniforms().binding() else {
            return Ok(());
        };

        let post_process = view_target.post_process_write();
        let bind_group_0 = {
            let view_uniforms_resource = world.resource::<ViewUniforms>();
            let view_uniforms = &view_uniforms_resource.uniforms;
            render_context.render_device().create_bind_group(
                "ray_trace_bind_group_0",
                &ray_trace_pipeline.layout_0,
                &BindGroupEntries::sequential((
                    view_uniforms,
                    globals_uniforms.clone(),
                    settings_binding.clone(),
                )),
            )
        };
        let (bind_group_1, bind_group_2) = {
            let Some(meta) = world.get_resource::<RayTraceMeta>() else {
                println!("No RayTraceMeta");
                return Ok(());
            };

            (
                render_context.render_device().create_bind_group(
                    "ray_trace_bind_group_1",
                    &ray_trace_pipeline.layout_1,
                    &BindGroupEntries::sequential((
                        meta.objects.binding().unwrap(),
                        meta.emissives.binding().unwrap(),
                    )),
                ),
                render_context.render_device().create_bind_group(
                    "ray_trace_bind_group_2",
                    &ray_trace_pipeline.layout_2,
                    &BindGroupEntries::sequential((
                        meta.materials.binding().unwrap(),
                        meta.meshes.binding().unwrap(),
                        meta.indices.binding().unwrap(),
                        meta.vertices.binding().unwrap(),
                        meta.textures.binding().unwrap(),
                        meta.texture_data.binding().unwrap(),
                    )),
                ),
            )
        };

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("ray_trace_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post_process.destination,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_render_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group_0, &[view_uniform_offset.offset]);
        render_pass.set_bind_group(1, &bind_group_1, &[]);
        render_pass.set_bind_group(2, &bind_group_2, &[]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}

#[derive(Resource)]
struct RayTracePipeline {
    layout_0: BindGroupLayout,
    layout_1: BindGroupLayout,
    layout_2: BindGroupLayout,
    pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for RayTracePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let layout_0 = render_device.create_bind_group_layout(
            "ray_trace_bind_group_layout_0",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    uniform_buffer::<ViewUniform>(true),
                    uniform_buffer::<GlobalsUniform>(false),
                    uniform_buffer::<RayTraceSettings>(false),
                ),
            ),
        );
        let layout_1 = render_device.create_bind_group_layout(
            "ray_trace_bind_group_layout_1",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: Some(Vec::<data::Object>::min_size()),
                    },
                    BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: Some(Vec::<u32>::min_size()),
                    },
                ),
            ),
        );
        let layout_2 = render_device.create_bind_group_layout(
            "ray_trace_bind_group_layout_2",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: Some(Vec::<data::Material>::min_size()),
                    },
                    BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: Some(Vec::<data::Mesh>::min_size()),
                    },
                    BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: Some(Vec::<u32>::min_size()),
                    },
                    BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: Some(Vec::<Vertex>::min_size()),
                    },
                    BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: Some(Vec::<Texture>::min_size()),
                    },
                    BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: Some(Vec::<f32>::min_size()),
                    },
                ),
            ),
        );

        let shader = world
            .resource_mut::<AssetServer>()
            .load("shaders/raytrace.wgsl");

        let pipeline_id =
            world
                .resource_mut::<PipelineCache>()
                .queue_render_pipeline(RenderPipelineDescriptor {
                    label: Some("ray_trace_pipeline".into()),
                    layout: vec![layout_0.clone(), layout_1.clone(), layout_2.clone()],
                    vertex: fullscreen_shader_vertex_state(),
                    fragment: Some(FragmentState {
                        shader,
                        shader_defs: vec![],
                        entry_point: "fragment".into(),
                        targets: vec![Some(ColorTargetState {
                            format: ViewTarget::TEXTURE_FORMAT_HDR, // TODO: support both HDR and SDR
                            blend: None,
                            write_mask: ColorWrites::ALL,
                        })],
                    }),
                    primitive: PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: MultisampleState::default(),
                    push_constant_ranges: vec![],
                });

        Self {
            layout_0,
            layout_1,
            layout_2,
            pipeline_id,
        }
    }
}
