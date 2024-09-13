use bevy::{
    asset::load_internal_asset,
    core_pipeline::{
        core_3d::graph::{Core3d, Node3d},
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
        prepass::ViewPrepassTextures,
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
        renderer::RenderDevice,
        view::{ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
        RenderApp,
    },
    utils::HashMap,
};

use crate::{
    data::{self, RayTraceMeta, RayTraceSettings, Texture, Vertex},
    extract,
};

const RT_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(11955195141264208704);

pub struct RayTracePlugin;

impl Plugin for RayTracePlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, RT_SHADER_HANDLE, "raytrace.wgsl", Shader::from_wgsl);

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

            handle_to_mesh: HashMap::new(),
            meshes: StorageBuffer::default(),
            indices: StorageBuffer::default(),
            vertices: StorageBuffer::default(),

            handle_to_material: HashMap::new(),
            handle_to_texture: HashMap::new(),
            materials: StorageBuffer::default(),
            textures: StorageBuffer::default(),
            texture_data: StorageBuffer::default(),
        });

        render_app.add_systems(
            ExtractSchedule,
            (
                (
                    extract::extract_meshes,
                    (extract::extract_textures, extract::extract_materials).chain(),
                ),
                extract::extract_visible,
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
                &BindGroupEntries::sequential((view_uniforms, globals_uniforms, settings_binding)),
            )
        };
        let (bind_group_1, bind_group_meshes, bind_group_materials) = {
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
                    "ray_trace_bind_group_meshes",
                    &ray_trace_pipeline.layout_meshes,
                    &BindGroupEntries::sequential((
                        meta.meshes.binding().unwrap(),
                        meta.indices.binding().unwrap(),
                        meta.vertices.binding().unwrap(),
                    )),
                ),
                render_context.render_device().create_bind_group(
                    "ray_trace_bind_group_materials",
                    &ray_trace_pipeline.layout_materials,
                    &BindGroupEntries::sequential((
                        meta.materials.binding().unwrap(),
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
        render_pass.set_bind_group(2, &bind_group_meshes, &[]);
        render_pass.set_bind_group(3, &bind_group_materials, &[]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}

#[derive(Resource)]
struct RayTracePipeline {
    layout_0: BindGroupLayout,
    layout_1: BindGroupLayout,
    layout_meshes: BindGroupLayout,
    layout_materials: BindGroupLayout,
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
        let layout_meshes = render_device.create_bind_group_layout(
            "ray_trace_bind_group_layout_meshes",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
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
                ),
            ),
        );
        let layout_materials = render_device.create_bind_group_layout(
            "ray_trace_bind_group_layout_materials",
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
                    layout: vec![
                        layout_0.clone(),
                        layout_1.clone(),
                        layout_meshes.clone(),
                        layout_materials.clone(),
                    ],
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
            layout_meshes,
            layout_materials,
            pipeline_id,
        }
    }
}
