// Based on bevy/examples/stress_tests/many_foxes.rs. Removes a bunch of options to keep
// things simple. Adds skinned aabb debug rendering - press J for joints and M for meshes.

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    input::common_conditions::input_just_pressed,
    prelude::*,
    window::{PresentMode, WindowResolution},
    winit::{UpdateMode, WinitSettings},
};
use bevy_mod_skinned_aabb::prelude::*;
use std::f32::consts::PI;

#[derive(Resource)]
struct Foxes {
    count: usize,
    speed: f32,
    moving: bool,
    sync: bool,
}

const NUM_FOXES: usize = 1000;

// Magic numbers from Fox.glb.
const NUM_JOINTS: usize = 24;
const NUM_SKINNED_JOINTS: usize = 22;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "🦊🦊🦊 Many Foxes! 🦊🦊🦊".into(),
                    present_mode: PresentMode::AutoNoVsync,
                    resolution: WindowResolution::new(1920.0, 1080.0)
                        .with_scale_factor_override(1.0),
                    ..default()
                }),
                ..default()
            }),
            FrameTimeDiagnosticsPlugin,
            LogDiagnosticsPlugin::default(),
        ))
        .add_plugins((
            SkinnedAabbPlugin,
            SkinnedAabbDebugPlugin::disable_by_default(),
        ))
        .insert_resource(WinitSettings {
            focused_mode: UpdateMode::Continuous,
            unfocused_mode: UpdateMode::Continuous,
        })
        .insert_resource(Foxes {
            count: NUM_FOXES,
            speed: 2.0,
            moving: true,
            sync: false,
        })
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 2000.0,
        })
        .add_systems(Startup, setup)
        .add_systems(Update, (setup_scene_once_loaded, update_fox_rings))
        .add_systems(
            Update,
            (
                toggle_draw_joint_aabbs.run_if(input_just_pressed(KeyCode::KeyJ)),
                toggle_draw_mesh_aabbs.run_if(input_just_pressed(KeyCode::KeyM)),
            ),
        )
        .run();
}

#[derive(Resource)]
struct Animations {
    node_indices: Vec<AnimationNodeIndex>,
    graph: Handle<AnimationGraph>,
}

const RING_SPACING: f32 = 2.0;
const FOX_SPACING: f32 = 2.0;

#[derive(Component, Clone, Copy)]
enum RotationDirection {
    CounterClockwise,
    Clockwise,
}

impl RotationDirection {
    fn sign(&self) -> f32 {
        match self {
            RotationDirection::CounterClockwise => 1.0,
            RotationDirection::Clockwise => -1.0,
        }
    }
}

#[derive(Component)]
struct Ring {
    radius: f32,
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
    foxes: Res<Foxes>,
) {
    // Insert a resource with the current scene information
    let animation_clips = [
        asset_server.load(GltfAssetLabel::Animation(2).from_asset("Fox.glb")),
        asset_server.load(GltfAssetLabel::Animation(1).from_asset("Fox.glb")),
        asset_server.load(GltfAssetLabel::Animation(0).from_asset("Fox.glb")),
    ];
    let mut animation_graph = AnimationGraph::new();
    let node_indices = animation_graph
        .add_clips(animation_clips.iter().cloned(), 1.0, animation_graph.root)
        .collect();
    commands.insert_resource(Animations {
        node_indices,
        graph: animation_graphs.add(animation_graph),
    });

    // Foxes
    // Concentric rings of foxes, running in opposite directions. The rings are spaced at 2m radius intervals.
    // The foxes in each ring are spaced at least 2m apart around its circumference.'

    // NOTE: This fox model faces +z
    let fox_handle = asset_server.load(GltfAssetLabel::Scene(0).from_asset("Fox.glb"));

    let ring_directions = [
        (
            Quat::from_rotation_y(PI),
            RotationDirection::CounterClockwise,
        ),
        (Quat::IDENTITY, RotationDirection::Clockwise),
    ];

    let mut ring_index = 0;
    let mut radius = RING_SPACING;
    let mut foxes_remaining = foxes.count;

    info!(
        "Spawning {} foxes, for a total of {} joints and {} skinned joints...",
        foxes.count,
        foxes.count * NUM_JOINTS,
        foxes.count * NUM_SKINNED_JOINTS
    );

    while foxes_remaining > 0 {
        let (base_rotation, ring_direction) = ring_directions[ring_index % 2];
        let ring_parent = commands
            .spawn((
                Transform::default(),
                Visibility::default(),
                ring_direction,
                Ring { radius },
            ))
            .id();

        let circumference = PI * 2. * radius;
        let foxes_in_ring = ((circumference / FOX_SPACING) as usize).min(foxes_remaining);
        let fox_spacing_angle = circumference / (foxes_in_ring as f32 * radius);

        for fox_i in 0..foxes_in_ring {
            let fox_angle = fox_i as f32 * fox_spacing_angle;
            let (s, c) = ops::sin_cos(fox_angle);
            let (x, z) = (radius * c, radius * s);

            commands.entity(ring_parent).with_children(|builder| {
                builder.spawn((
                    SceneRoot(fox_handle.clone()),
                    Transform::from_xyz(x, 0.0, z)
                        .with_scale(Vec3::splat(0.01))
                        .with_rotation(base_rotation * Quat::from_rotation_y(-fox_angle)),
                ));
            });
        }

        foxes_remaining -= foxes_in_ring;
        radius += RING_SPACING;
        ring_index += 1;
    }

    // Camera
    let zoom = 0.8;
    let translation = Vec3::new(
        radius * 1.25 * zoom,
        radius * 0.5 * zoom,
        radius * 1.5 * zoom,
    );
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(translation)
            .looking_at(0.2 * Vec3::new(translation.x, 0.0, translation.z), Vec3::Y),
    ));

    // Plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(5000.0, 5000.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
    ));

    // Light
    commands.spawn((
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, 1.0, -PI / 4.)),
        DirectionalLight {
            shadows_enabled: false,
            ..default()
        },
    ));
}

// Once the scene is loaded, start the animation
fn setup_scene_once_loaded(
    animations: Res<Animations>,
    foxes: Res<Foxes>,
    mut commands: Commands,
    mut player: Query<(Entity, &mut AnimationPlayer)>,
    mut done: Local<bool>,
) {
    if !*done && player.iter().len() == foxes.count {
        for (entity, mut player) in &mut player {
            commands
                .entity(entity)
                .insert(AnimationGraphHandle(animations.graph.clone()))
                .insert(AnimationTransitions::new());

            let playing_animation = player.play(animations.node_indices[0]).repeat();
            if !foxes.sync {
                playing_animation.seek_to(entity.index() as f32 / 10.0);
            }
        }
        *done = true;
    }
}

fn update_fox_rings(
    time: Res<Time>,
    foxes: Res<Foxes>,
    mut rings: Query<(&Ring, &RotationDirection, &mut Transform)>,
) {
    if !foxes.moving {
        return;
    }

    let dt = time.delta_secs();
    for (ring, rotation_direction, mut transform) in &mut rings {
        let angular_velocity = foxes.speed / ring.radius;
        transform.rotate_y(rotation_direction.sign() * angular_velocity * dt);
    }
}
