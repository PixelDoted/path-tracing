mod common;

use bevy::{
    core_pipeline::{
        bloom::BloomSettings,
        experimental::taa::{TemporalAntiAliasBundle, TemporalAntiAliasPlugin},
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
        .add_systems(Update, (get_origin, sinwave))
        .run();
}

fn setup(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    let (samples, bounces) = common::get_settings();

    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                hdr: true,
                clear_color: ClearColorConfig::Custom(Color::linear_rgb(0.1, 0.2, 0.4)),
                // is_active: false,
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
            sky_color: Color::linear_rgb(0.1, 0.2, 0.4).into(),
        },
        TemporalAntiAliasBundle::default(),
    ));

    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));

    commands.spawn((
        PbrBundle {
            mesh: cube.clone(),
            material: materials.add(StandardMaterial {
                base_color: Color::linear_rgb(0.0, 0.0, 1.0),
                perceptual_roughness: 0.5,
                ..default()
            }),
            transform: Transform::from_xyz(0.0, 0.0, 1.5),
            ..default()
        },
        SinWave(Vec3::Y * 1.0),
    ));

    commands.spawn((
        PbrBundle {
            mesh: cube.clone(),
            material: materials.add(StandardMaterial {
                base_color: Color::linear_rgb(1.0, 0.0, 0.0),
                perceptual_roughness: 1.0,
                ..default()
            }),
            transform: Transform::from_xyz(0.0, 0.0, -1.5),
            ..default()
        },
        SinWave(Vec3::Y * -1.0),
    ));

    commands.spawn((
        PbrBundle {
            mesh: cube.clone(),
            material: materials.add(StandardMaterial {
                base_color: Color::linear_rgb(0.0, 0.0, 0.0),
                emissive: Color::linear_rgb(2.0, 2.0, 2.0).into(),
                ..default()
            }),
            transform: Transform::from_xyz(1.5, 0.0, 0.0).with_scale(Vec3::new(0.5, 0.5, 2.0)),
            ..default()
        },
        SinWave(Vec3::Y * -0.6),
    ));

    commands.spawn((
        PbrBundle {
            mesh: cube.clone(),
            material: materials.add(StandardMaterial {
                base_color: Color::linear_rgb(0.0, 0.0, 0.0),
                emissive: Color::linear_rgb(2.0, 1.7, 0.0).into(),
                ..default()
            }),
            transform: Transform::from_xyz(-1.5, 0.0, 0.0).with_scale(Vec3::new(0.5, 2.0, 0.5)),
            ..default()
        },
        SinWave(Vec3::Z * 0.6),
    ));

    commands.spawn((PbrBundle {
        mesh: cube.clone(),
        material: materials.add(StandardMaterial {
            base_color: Color::linear_rgb(0.0, 1.0, 0.0),
            perceptual_roughness: 0.0,
            metallic: 0.1,
            ..default()
        }),
        transform: Transform::from_scale(Vec3::new(0.5, 0.5, 0.5)).with_rotation(Quat::from_euler(
            EulerRot::XYZ,
            45f32.to_radians(),
            45f32.to_radians(),
            0.0,
        )),
        ..default()
    },));

    commands.spawn(PbrBundle {
        mesh: meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(5.0))),
        material: materials.add(StandardMaterial {
            base_color: Color::linear_rgb(0.4, 0.4, 0.4),
            perceptual_roughness: 0.1,
            metallic: 1.0,
            ..default()
        }),
        transform: Transform::from_xyz(0.0, -2.0, 0.0),
        ..default()
    });
}

#[derive(Component)]
pub struct SinWave(pub Vec3);

fn sinwave(time: Res<Time>, mut query: Query<(&SinWave, &Origin, &mut Transform)>) {
    for (sinwave, origin, mut transform) in query.iter_mut() {
        transform.translation =
            origin.0 + (time.elapsed_seconds() * sinwave.0.length()).sin() * sinwave.0.normalize();
    }
}

#[derive(Component)]
pub struct Origin(pub Vec3);

fn get_origin(query: Query<(Entity, &Transform), Without<Origin>>, mut commands: Commands) {
    for (entity, transform) in query.iter() {
        commands
            .entity(entity)
            .insert(Origin(transform.translation));
    }
}
