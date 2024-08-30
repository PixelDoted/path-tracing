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
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let (samples, bounces) = common::get_settings();

    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                hdr: true,
                clear_color: ClearColorConfig::Custom(Color::BLACK),
                // is_active: false,
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 7.0),
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
        TemporalAntiAliasBundle::default(),
    ));

    let scene = asset_server.load::<Scene>("scene.glb#Scene0");
    commands.spawn(SceneBundle { scene, ..default() });
}
