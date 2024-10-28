use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};

/// Get samples and bounces
pub fn get_settings() -> (u32, u32) {
    let env_samples = std::env::var("RT_SAMPLES")
        .map(|s| s.parse::<u32>().ok())
        .ok()
        .flatten();
    let env_bounces = std::env::var("RT_BOUNCES")
        .map(|s| s.parse::<u32>().ok())
        .ok()
        .flatten();

    let samples = env_samples.unwrap_or(2);
    let bounces = env_bounces.unwrap_or(10);

    log::info!("Path Tracing Samples: {}, Bounces: {}", samples, bounces);
    (samples, bounces)
}

#[derive(Component, Default)]
pub struct FlyCam {
    pub look_vector: Vec2,
    pub sensitivity: f32,
    pub speed: f32,
}

fn setup(mut query: Query<(&Transform, &mut FlyCam), Added<FlyCam>>) {
    for (transform, mut flycam) in query.iter_mut() {
        let (y, x, _z) = transform.rotation.to_euler(EulerRot::YXZ);
        flycam.look_vector = Vec2::new(x, y);
    }
}

fn update(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut query: Query<(&mut Transform, &mut FlyCam)>,
    mut window_query: Query<&mut Window, With<PrimaryWindow>>,
) {
    let grab_cursor = mouse_button.pressed(MouseButton::Right);
    if grab_cursor {
        let x = keys.pressed(KeyCode::KeyD) as i8 - keys.pressed(KeyCode::KeyA) as i8;
        let y = keys.pressed(KeyCode::KeyE) as i8 - keys.pressed(KeyCode::KeyQ) as i8;
        let z = keys.pressed(KeyCode::KeyW) as i8 - keys.pressed(KeyCode::KeyS) as i8;
        let mouse_delta = mouse_motion.read().fold(Vec2::ZERO, |mut a, b| {
            a.x -= b.delta.y.to_radians();
            a.y -= b.delta.x.to_radians();
            a
        });

        for (mut transform, mut flycam) in query.iter_mut() {
            let look_delta = mouse_delta * flycam.sensitivity;
            flycam.look_vector += look_delta;
            transform.rotation = Quat::from_euler(
                EulerRot::YXZ,
                flycam.look_vector.y,
                flycam.look_vector.x,
                0.0,
            );

            let right = transform.right() * x as f32;
            let up = transform.up() * y as f32;
            let forward = transform.forward() * z as f32;
            transform.translation += (right + up + forward) * flycam.speed * time.delta_secs();
        }
    }

    let Ok(mut window) = window_query.get_single_mut() else {
        return;
    };
    if grab_cursor {
        if window.cursor_options.visible {
            window.cursor_options.visible = false;
            window.cursor_options.grab_mode = CursorGrabMode::Locked;
        }
    } else if !window.cursor_options.visible {
        window.cursor_options.visible = true;
        window.cursor_options.grab_mode = CursorGrabMode::None;
    }
}

pub struct FlyCamPlugin;
impl Plugin for FlyCamPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (setup, update).chain());
    }
}
