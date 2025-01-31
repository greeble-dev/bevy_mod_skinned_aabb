#[path = "../dev/dev.rs"]
mod dev;

use bevy::{input::common_conditions::input_just_pressed, prelude::*};
use bevy_mod_skinned_aabb::{
    debug::{toggle_draw_joint_aabbs, toggle_draw_mesh_aabbs, SkinnedAabbDebugPlugin},
    SkinnedAabbPlugin,
};
use dev::{spawn_random_mesh_selection, update_random_mesh_animations};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Skinned AABB Random Meshes".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(SkinnedAabbPlugin)
        .add_plugins(SkinnedAabbDebugPlugin::enable_by_default())
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 2000.0,
        })
        .add_systems(
            Update,
            (
                toggle_pause.run_if(input_just_pressed(KeyCode::Space)),
                toggle_draw_joint_aabbs.run_if(input_just_pressed(KeyCode::KeyJ)),
                toggle_draw_mesh_aabbs.run_if(input_just_pressed(KeyCode::KeyM)),
            ),
        )
        .add_systems(Startup, setup)
        .add_systems(Startup, spawn_random_mesh_selection)
        .add_systems(Update, update_random_mesh_animations)
        .run();
}

fn toggle_pause(mut time: ResMut<Time<Virtual>>) {
    if time.is_paused() {
        time.unpause();
    } else {
        time.pause();
    }
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 1.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Text::new("J: Toggle Joint AABBs\nM: Toggle Mesh AABBs"),
        TextFont {
            font_size: 15.0,
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}
