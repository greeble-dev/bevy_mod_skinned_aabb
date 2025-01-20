use bevy::{
    input::common_conditions::input_just_pressed, prelude::*,
    render::mesh::skinning::SkinnedMeshInverseBindposes,
};
use bevy_mod_skinned_aabb::{
    debug::{toggle_draw_joint_aabbs, toggle_draw_mesh_aabbs, SkinnedAabbDebugPlugin},
    dev::{
        create_and_spawn_random_skinned_mesh, random_vec3_snorm, update_random_meshes,
        RandomSkinnedMeshType,
    },
    SkinnedAabbPlugin,
};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

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
        .add_systems(Startup, spawn_random_meshes)
        .add_systems(Update, update_random_meshes)
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

fn spawn_random_meshes(
    mut commands: Commands,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
    mut inverse_bindposes_assets: ResMut<Assets<SkinnedMeshInverseBindposes>>,
) {
    let mut rng = ChaCha8Rng::seed_from_u64(732935);

    let material = MeshMaterial3d(material_assets.add(StandardMaterial {
        base_color: Color::WHITE,
        cull_mode: None,
        ..default()
    }));

    struct MeshInstance {
        mesh_type: RandomSkinnedMeshType,
        num_joints: usize,
        translation: Vec3,
    }

    let mesh_instances = [
        MeshInstance {
            mesh_type: RandomSkinnedMeshType::Hard,
            num_joints: 1,
            translation: Vec3::new(-3.0, 1.5, 0.0),
        },
        MeshInstance {
            mesh_type: RandomSkinnedMeshType::Hard,
            num_joints: 20,
            translation: Vec3::new(0.0, 1.5, 0.0),
        },
        MeshInstance {
            mesh_type: RandomSkinnedMeshType::Hard,
            num_joints: 200,
            translation: Vec3::new(3.0, 1.5, 0.0),
        },
        MeshInstance {
            mesh_type: RandomSkinnedMeshType::Soft { num_tris: 100 },
            num_joints: 1,
            translation: Vec3::new(-3.0, -1.5, 0.0),
        },
        MeshInstance {
            mesh_type: RandomSkinnedMeshType::Soft { num_tris: 100 },
            num_joints: 20,
            translation: Vec3::new(0.0, -1.5, 0.0),
        },
        MeshInstance {
            mesh_type: RandomSkinnedMeshType::Soft { num_tris: 100 },
            num_joints: 200,
            translation: Vec3::new(3.0, -1.5, 0.0),
        },
    ];

    for mesh_instance in mesh_instances {
        // Create a base entity. This will be the parent of the mesh and the joints.

        let base_transform = Transform::from_translation(mesh_instance.translation);
        let base_entity = commands.spawn((base_transform, Visibility::default())).id();

        // Give the mesh entity a random translation. This ensures we're not depending on the
        // mesh having the same transform as the root joint.

        let mesh_transform = Transform::from_translation(random_vec3_snorm(&mut rng));

        if let Ok(entity) = create_and_spawn_random_skinned_mesh(
            &mut commands,
            &mut mesh_assets,
            &mut inverse_bindposes_assets,
            &mut rng,
            base_entity,
            mesh_transform,
            mesh_instance.mesh_type,
            mesh_instance.num_joints,
        ) {
            commands.entity(entity).insert(material.clone());
        }
    }
}
