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

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let (samples, bounces) = common::get_settings();

    commands.spawn((
        Camera3d::default(),
        Camera {
            hdr: true,
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            // is_active: false,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 7.0),
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

    let scene = asset_server.load::<Scene>("scene.glb#Scene0");
    commands.spawn(SceneRoot(scene));
}
