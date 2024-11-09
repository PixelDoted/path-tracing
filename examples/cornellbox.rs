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
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    let (samples, bounces) = common::get_settings();

    commands.insert_resource(AmbientLight {
        color: Color::linear_rgb(0.1, 0.2, 0.4),
        ..default()
    });
    commands.spawn((
        Camera3d::default(),
        Camera {
            hdr: true,
            clear_color: ClearColorConfig::Custom(Color::linear_rgb(0.1, 0.2, 0.4)),
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
            sky_color: Color::BLACK.into(),
        },
        TemporalAntiAliasing::default(),
        Msaa::Off,
    ));

    let white = materials.add(StandardMaterial {
        base_color: Color::linear_rgb(1.0, 1.0, 1.0),
        ..default()
    });
    let red = materials.add(StandardMaterial {
        base_color: Color::linear_rgb(1.0, 0.0, 0.0),
        ..default()
    });
    let green = materials.add(StandardMaterial {
        base_color: Color::linear_rgb(0.0, 1.0, 0.0),
        ..default()
    });
    let light = materials.add(StandardMaterial {
        base_color: Color::BLACK,
        emissive: LinearRgba::rgb(1.0, 1.0, 1.0),
        ..default()
    });

    commands.spawn_batch([
        (
            Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(1.1)))),
            MeshMaterial3d(white.clone()),
            Transform::from_xyz(0.0, -1.0, 0.0),
        ),
        (
            Mesh3d(meshes.add(Plane3d::new(Vec3::NEG_Y, Vec2::splat(1.1)))),
            MeshMaterial3d(white.clone()),
            Transform::from_xyz(0.0, 1.0, 0.0),
        ),
        (
            Mesh3d(meshes.add(Plane3d::new(Vec3::Z, Vec2::splat(1.1)))),
            MeshMaterial3d(white.clone()),
            Transform::from_xyz(0.0, 0.0, -1.0),
        ),
        (
            Mesh3d(meshes.add(Plane3d::new(Vec3::NEG_Z, Vec2::splat(1.1)))),
            MeshMaterial3d(white.clone()),
            Transform::from_xyz(0.0, 0.0, 1.0),
        ),
        (
            Mesh3d(meshes.add(Plane3d::new(Vec3::X, Vec2::splat(1.1)))),
            MeshMaterial3d(red.clone()),
            Transform::from_xyz(-1.0, 0.0, 0.0),
        ),
        (
            Mesh3d(meshes.add(Plane3d::new(Vec3::NEG_X, Vec2::splat(1.1)))),
            MeshMaterial3d(green.clone()),
            Transform::from_xyz(1.0, 0.0, 0.0),
        ),
        (
            Mesh3d(meshes.add(Plane3d::new(Vec3::NEG_Y, Vec2::splat(0.25)))),
            MeshMaterial3d(light.clone()),
            Transform::from_xyz(0.0, 0.95, 0.0),
        ),
    ]);
}
