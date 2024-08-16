mod data;
mod flycam;
mod shader;

use bevy::{core_pipeline::bloom::BloomSettings, prelude::*};
use data::RayTraceSettings;
use flycam::{FlyCam, FlyCamPlugin};
use shader::RayTracePlugin;

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, RayTracePlugin, FlyCamPlugin));
    app.add_systems(Startup, setup);
    app.add_systems(Update, (get_origin, sinwave, toggle_raytrace));
    app.run();
}

fn setup(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
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
    ));

    // ---- Cornell Box ----
    let white = materials.add(Color::linear_rgb(1.0, 1.0, 1.0));
    let red = materials.add(Color::linear_rgb(1.0, 0.0, 0.0));
    let green = materials.add(Color::linear_rgb(0.0, 1.0, 0.0));
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

    // ---- Env ----
    // let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));

    // commands.spawn((
    //     PbrBundle {
    //         mesh: cube.clone(),
    //         material: materials.add(StandardMaterial {
    //             base_color: Color::linear_rgb(0.0, 0.0, 1.0),
    //             perceptual_roughness: 0.0,
    //             ..default()
    //         }),
    //         transform: Transform::from_xyz(0.0, 0.0, 1.5),
    //         ..default()
    //     },
    //     SinWave(Vec3::Y * 1.0),
    // ));

    // commands.spawn((
    //     PbrBundle {
    //         mesh: cube.clone(),
    //         material: materials.add(StandardMaterial {
    //             base_color: Color::linear_rgb(1.0, 0.0, 0.0),
    //             perceptual_roughness: 0.0,
    //             ..default()
    //         }),
    //         transform: Transform::from_xyz(0.0, 0.0, -1.5),
    //         ..default()
    //     },
    //     SinWave(Vec3::Y * -1.0),
    // ));

    // commands.spawn((
    //     PbrBundle {
    //         mesh: cube.clone(),
    //         material: materials.add(StandardMaterial {
    //             base_color: Color::linear_rgb(0.0, 0.0, 0.0),
    //             emissive: Color::linear_rgb(2.0, 2.0, 2.0).into(),
    //             ..default()
    //         }),
    //         transform: Transform::from_xyz(1.5, 0.0, 0.0).with_scale(Vec3::new(0.5, 0.5, 2.0)),
    //         ..default()
    //     },
    //     SinWave(Vec3::Y * -0.6),
    // ));

    // commands.spawn((
    //     PbrBundle {
    //         mesh: cube.clone(),
    //         material: materials.add(StandardMaterial {
    //             base_color: Color::linear_rgb(0.0, 0.0, 0.0),
    //             emissive: Color::linear_rgb(2.0, 1.7, 0.0).into(),
    //             ..default()
    //         }),
    //         transform: Transform::from_xyz(-1.5, 0.0, 0.0).with_scale(Vec3::new(0.5, 2.0, 0.5)),
    //         ..default()
    //     },
    //     SinWave(Vec3::Z * 0.6),
    // ));

    // commands.spawn((PbrBundle {
    //     mesh: cube.clone(),
    //     material: materials.add(StandardMaterial {
    //         base_color: Color::linear_rgb(0.9, 0.9, 0.9),
    //         perceptual_roughness: 0.0,
    //         ..default()
    //     }),
    //     transform: Transform::from_scale(Vec3::new(0.5, 0.5, 0.5)).with_rotation(Quat::from_euler(
    //         EulerRot::XYZ,
    //         45f32.to_radians(),
    //         45f32.to_radians(),
    //         0.0,
    //     )),
    //     ..default()
    // },));
}

fn toggle_raytrace(
    keys: Res<ButtonInput<KeyCode>>,
    mut query: Query<(Entity, Has<RayTraceSettings>), With<Camera3d>>,
    mut commands: Commands,
) {
    if !keys.just_pressed(KeyCode::Space) {
        return;
    }

    for (entity, has_raytrace) in query.iter_mut() {
        if has_raytrace {
            commands.entity(entity).remove::<RayTraceSettings>();
        } else {
            commands.entity(entity).insert(RayTraceSettings {
                bounces: 4,
                samples: 1,
                fov: std::f32::consts::FRAC_PI_4,
                sky_color: Color::BLACK.into(),
                // sky_color: Color::linear_rgb(0.1, 0.2, 0.4).into(),
            });
        }
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

#[derive(Component)]
pub struct SinWave(pub Vec3);

fn sinwave(time: Res<Time>, mut query: Query<(&SinWave, &Origin, &mut Transform)>) {
    for (sinwave, origin, mut transform) in query.iter_mut() {
        transform.translation =
            origin.0 + (time.elapsed_seconds() * sinwave.0.length()).sin() * sinwave.0.normalize();
    }
}
