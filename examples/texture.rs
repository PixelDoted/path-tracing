mod common;

use bevy::{
    core_pipeline::{
        bloom::Bloom,
        experimental::taa::{TemporalAntiAliasPlugin, TemporalAntiAliasing},
    },
    prelude::*,
};
use common::{FlyCam, FlyCamPlugin};
use path_tracing::{RayTracePlugin, RayTraceSettings};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            RayTracePlugin,
            FlyCamPlugin,
            TemporalAntiAliasPlugin,
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    let (samples, bounces) = common::get_settings();

    commands.spawn((
        Camera3d::default(),
        Camera {
            hdr: true,
            clear_color: ClearColorConfig::Custom(Color::linear_rgb(0.1, 0.2, 0.4)),
            // is_active: false,
            ..default()
        },
        Transform::from_xyz(3.0, 3.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
        FlyCam {
            speed: 6.0,
            sensitivity: 0.1,
            ..default()
        },
        Bloom::default(),
        RayTraceSettings {
            bounces,
            samples,
            fov: std::f32::consts::FRAC_PI_4,
            sky_color: Color::linear_rgb(0.5, 0.5, 0.5).into(),
        },
        TemporalAntiAliasing::default(),
        Msaa::Off,
    ));

    let texture = asset_server.load("example.png");

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::linear_rgb(1.0, 1.0, 1.0),
            base_color_texture: Some(texture),
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
}
