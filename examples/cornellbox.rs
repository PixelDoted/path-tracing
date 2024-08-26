mod common;

use bevy::{core_pipeline::bloom::BloomSettings, prelude::*};
use common::{FlyCam, FlyCamPlugin};
use path_tracing::{RayTracePlugin, RayTraceSettings};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, RayTracePlugin, FlyCamPlugin))
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
        Camera3dBundle {
            camera: Camera {
                hdr: true,
                clear_color: ClearColorConfig::Custom(Color::linear_rgb(0.1, 0.2, 0.4)),
                ..default()
            },
            transform: Transform::from_xyz(3.0, 3.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        FlyCam {
            speed: 6.0,
            sensitivity: 0.1,
            ..default()
        },
        BloomSettings::default(),
        RayTraceSettings {
            bounces,
            samples,
            fov: std::f32::consts::FRAC_PI_4,
            sky_color: Color::BLACK.into(),
        },
    ));

    let white = materials.add(Color::linear_rgb(1.0, 1.0, 1.0));
    let red = materials.add(Color::linear_rgb(1.0, 0.0, 0.0));
    let green = materials.add(StandardMaterial {
        base_color: Color::linear_rgb(0.0, 1.0, 0.0),
        metallic: 1.0,
        perceptual_roughness: 0.0,
        ..default()
    });
    let light = materials.add(StandardMaterial {
        base_color: Color::BLACK,
        emissive: LinearRgba::rgb(1.0, 1.0, 1.0),
        ..default()
    });

    commands.spawn_batch([
        PbrBundle {
            mesh: meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(1.1))),
            material: white.clone(),
            transform: Transform::from_xyz(0.0, -1.0, 0.0),
            ..default()
        },
        PbrBundle {
            mesh: meshes.add(Plane3d::new(Vec3::NEG_Y, Vec2::splat(1.1))),
            material: white.clone(),
            transform: Transform::from_xyz(0.0, 1.0, 0.0),
            ..default()
        },
        PbrBundle {
            mesh: meshes.add(Plane3d::new(Vec3::Z, Vec2::splat(1.1))),
            material: white.clone(),
            transform: Transform::from_xyz(0.0, 0.0, -1.0),
            ..default()
        },
        PbrBundle {
            mesh: meshes.add(Plane3d::new(Vec3::NEG_Z, Vec2::splat(1.1))),
            material: white.clone(),
            transform: Transform::from_xyz(0.0, 0.0, 1.0),
            ..default()
        },
        PbrBundle {
            mesh: meshes.add(Plane3d::new(Vec3::X, Vec2::splat(1.1))),
            material: red.clone(),
            transform: Transform::from_xyz(-1.0, 0.0, 0.0),
            ..default()
        },
        PbrBundle {
            mesh: meshes.add(Plane3d::new(Vec3::NEG_X, Vec2::splat(1.1))),
            material: green.clone(),
            transform: Transform::from_xyz(1.0, 0.0, 0.0),
            ..default()
        },
        PbrBundle {
            mesh: meshes.add(Plane3d::new(Vec3::NEG_Y, Vec2::splat(0.25))),
            material: light.clone(),
            transform: Transform::from_xyz(0.0, 0.95, 0.0),
            ..default()
        },
    ]);
}
