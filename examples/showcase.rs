use bevy::{
    asset::RenderAssetUsages,
    input::common_conditions::input_just_pressed,
    picking::{backend::ray::RayMap, mesh_picking::ray_cast::ray_aabb_intersection_3d},
    prelude::*,
    render::mesh::{
        PrimitiveTopology, VertexAttributeValues,
        skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
    },
    scene::SceneInstanceReady,
};
use bevy_math::{Affine3A, bounding::Aabb3d};
use bevy_mod_skinned_aabb::{SkinnedAabb, SkinnedAabbAsset, prelude::*};
use bevy_render::primitives::Aabb;
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Skinned AABB Showcase".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(SkinnedAabbPlugin)
        .add_plugins(SkinnedAabbDebugPlugin::enable_by_default())
        .insert_resource(AmbientLight {
            brightness: 2000.0,
            ..Default::default()
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
        .add_systems(Startup, load_gltf_mesh_assets)
        .add_systems(Startup, spawn_custom_meshes)
        .add_systems(Update, spawn_gltf_mesh_scenes)
        .add_systems(Update, update_custom_mesh_animation)
        .add_systems(Update, update_turntables)
        .add_systems(Update, raycast.after(TransformSystem::TransformPropagate))
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
        Transform::from_xyz(0.0, 7.5, 18.0).looking_at(Vec3::new(0.0, 5.5, 0.0), Vec3::Y),
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

#[derive(Copy, Clone, Debug, Default)]
struct GltfLayout {
    path: &'static str,
    transform: Transform,
    animation_name: &'static str,
    animation_speed: f32,
}

const GLTF_LAYOUTS: &[GltfLayout] = &[
    GltfLayout {
        path: "Fox.glb",
        transform: Transform::from_xyz(-4.75, 5.5, 0.0).with_scale(Vec3::splat(0.06)),
        animation_name: "Run",
        animation_speed: 0.8,
    },
    GltfLayout {
        path: "RecursiveSkeletons.glb",
        transform: Transform::from_xyz(7.0, 5.0, 0.0).with_scale(Vec3::splat(0.04)),
        animation_name: "Track0",
        animation_speed: 0.4,
    },
];

#[derive(Component, Debug, Default)]
struct GltfPendingAsset {
    handle: Handle<Gltf>,
    layout: GltfLayout,
}

#[derive(Component, Debug, Default)]
struct GltfPendingAnimation {
    graph_handle: Handle<AnimationGraph>,
    graph_node_index: AnimationNodeIndex,
    speed: f32,
}

fn load_gltf_mesh_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    for layout in GLTF_LAYOUTS {
        let pending = GltfPendingAsset {
            handle: asset_server.load(layout.path),
            layout: *layout,
        };

        commands.spawn((pending, layout.transform, Turntable));
    }
}

fn spawn_gltf_mesh_scenes(
    mut commands: Commands,
    query: Query<(Entity, &GltfPendingAsset)>,
    assets: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    for (entity, asset) in query.iter() {
        let Some(gltf) = assets.get(&asset.handle) else {
            continue;
        };

        commands.entity(entity).remove::<GltfPendingAsset>();

        let (Some(scene_handle), Some(animation_handle)) = (
            gltf.scenes.first(),
            gltf.named_animations.get(asset.layout.animation_name),
        ) else {
            continue;
        };

        let (graph, graph_node_index) = AnimationGraph::from_clip(animation_handle.clone());

        let animation = GltfPendingAnimation {
            graph_handle: graphs.add(graph),
            graph_node_index,
            speed: asset.layout.animation_speed,
        };

        commands
            .entity(entity)
            .insert((SceneRoot(scene_handle.clone()), animation))
            .observe(play_gltf_mesh_animations);
    }
}

fn play_gltf_mesh_animations(
    trigger: Trigger<SceneInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    animations: Query<&GltfPendingAnimation>,
    mut players: Query<&mut AnimationPlayer>,
) {
    commands
        .entity(trigger.target())
        .remove::<GltfPendingAnimation>();

    if let Ok(animation) = animations.get(trigger.target()) {
        for child in children.iter_descendants(trigger.target()) {
            if let Ok(mut player) = players.get_mut(child) {
                player
                    .play(animation.graph_node_index)
                    .set_speed(animation.speed)
                    .repeat();

                commands
                    .entity(child)
                    .insert(AnimationGraphHandle(animation.graph_handle.clone()));
            }
        }
    }
}

type CustomAnimationId = i8;

#[derive(Component)]
struct CustomAnimation(CustomAnimationId);

fn spawn_custom_meshes(
    mut commands: Commands,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
    mut inverse_bindposes_assets: ResMut<Assets<SkinnedMeshInverseBindposes>>,
) {
    // Adapted from bevy/examples/animation/custom_skinned_mesh.rs.

    let mesh_handle = mesh_assets.add(
        Mesh::new(
            PrimitiveTopology::TriangleStrip,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![
                [-0.5, 0.0, 0.0],
                [0.5, 0.0, 0.0],
                [-0.5, 0.5, 0.0],
                [0.5, 0.5, 0.0],
                [-0.5, 1.0, 0.0],
                [0.5, 1.0, 0.0],
                [-0.5, 1.5, 0.0],
                [0.5, 1.5, 0.0],
                [-0.5, 2.0, 0.0],
                [0.5, 2.0, 0.0],
            ],
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 10])
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_JOINT_INDEX,
            VertexAttributeValues::Uint16x4(vec![
                [1, 0, 0, 0],
                [1, 0, 0, 0],
                [1, 2, 0, 0],
                [1, 2, 0, 0],
                [1, 2, 0, 0],
                [1, 2, 0, 0],
                [2, 1, 0, 0],
                [2, 1, 0, 0],
                [2, 0, 0, 0],
                [2, 0, 0, 0],
            ]),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_JOINT_WEIGHT,
            vec![
                [1.00, 0.00, 0.0, 0.0],
                [1.00, 0.00, 0.0, 0.0],
                [0.75, 0.25, 0.0, 0.0],
                [0.75, 0.25, 0.0, 0.0],
                [0.50, 0.50, 0.0, 0.0],
                [0.50, 0.50, 0.0, 0.0],
                [0.75, 0.25, 0.0, 0.0],
                [0.75, 0.25, 0.0, 0.0],
                [1.00, 0.00, 0.0, 0.0],
                [1.00, 0.00, 0.0, 0.0],
            ],
        ),
    );

    let inverse_bindposes_handle = inverse_bindposes_assets.add(vec![
        Mat4::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        Mat4::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        Mat4::from_translation(Vec3::new(0.0, -1.0, 0.0)),
    ]);

    struct MeshInstance {
        animations: [CustomAnimationId; 2],
    }

    let mesh_instances = [
        // Simple cases. First joint is still, second joint is all rotation/translation/scale variations.
        MeshInstance { animations: [0, 1] },
        MeshInstance { animations: [0, 2] },
        MeshInstance { animations: [0, 3] },
        MeshInstance { animations: [0, 4] },
        MeshInstance { animations: [0, 5] },
        MeshInstance { animations: [0, 6] },
        MeshInstance { animations: [0, 7] },
        MeshInstance { animations: [0, 8] },
        // Skewed cases. First joint is non-uniform scaling, second joint is rotation/translation variations.
        MeshInstance { animations: [9, 1] },
        MeshInstance { animations: [9, 2] },
        MeshInstance { animations: [9, 3] },
        MeshInstance { animations: [9, 4] },
        MeshInstance { animations: [9, 5] },
    ];

    for (i, mesh_instance) in mesh_instances.iter().enumerate() {
        let x = ((i as f32) * 2.0) - ((mesh_instances.len() - 1) as f32);

        let base_entity = commands
            .spawn((Transform::from_xyz(x, 0.0, 0.0), Visibility::default()))
            .id();

        let joints = vec![
            commands.spawn((Transform::IDENTITY,)).id(),
            commands
                .spawn((
                    CustomAnimation(mesh_instance.animations[0]),
                    Transform::IDENTITY,
                ))
                .id(),
            commands
                .spawn((
                    CustomAnimation(mesh_instance.animations[1]),
                    Transform::IDENTITY,
                ))
                .id(),
        ];

        commands.entity(joints[0]).insert(ChildOf(base_entity));

        commands.entity(joints[1]).insert(ChildOf(joints[0]));
        commands.entity(joints[2]).insert(ChildOf(joints[1]));

        let mesh_entity = commands
            .spawn((
                Transform::IDENTITY,
                Mesh3d(mesh_handle.clone()),
                MeshMaterial3d(material_assets.add(StandardMaterial {
                    base_color: Color::WHITE,
                    cull_mode: None,
                    ..default()
                })),
                SkinnedMesh {
                    inverse_bindposes: inverse_bindposes_handle.clone(),
                    joints: joints.clone(),
                },
            ))
            .id();

        commands.entity(mesh_entity).insert(ChildOf(base_entity));
    }
}

fn update_custom_mesh_animation(
    time: Res<Time<Virtual>>,
    mut query: Query<(&mut Transform, &CustomAnimation)>,
) {
    let t = time.elapsed_secs();
    let ts = ops::sin(t);
    let tc = ops::cos(t);
    let ots = ops::sin(t + FRAC_PI_4);
    let otc = ops::cos(t + FRAC_PI_4);

    for (mut transform, animation) in &mut query {
        match animation.0 {
            1 => transform.translation = Vec3::new(0.5 * ts, 0.5 + tc, 0.0),
            2 => transform.translation = Vec3::new(0.0, 0.5 + ts, tc),
            3 => transform.rotation = Quat::from_rotation_x(FRAC_PI_2 * ts),
            4 => transform.rotation = Quat::from_rotation_y(FRAC_PI_2 * ts),
            5 => transform.rotation = Quat::from_rotation_z(FRAC_PI_2 * ts),
            6 => transform.scale.x = ts * 1.5,
            7 => transform.scale.y = ts * 1.5,
            8 => transform.scale = Vec3::new(ts * 1.5, otc * 1.5, 1.0),
            9 => transform.scale = Vec3::new(ots, 1.0 + (tc * 0.5), 1.0 - (tc * 0.5)),
            _ => (),
        }
    }
}

#[derive(Component, Debug)]
#[require(Visibility)]
struct Turntable;

fn update_turntables(mut query: Query<(&mut Transform, &Turntable)>, time: Res<Time<Virtual>>) {
    for (mut transform, _) in &mut query {
        transform.rotation = Quat::from_rotation_y(time.elapsed_secs() * 0.5);
    }
}

// TODO: Avoid duplication from debug.rs.
fn gizmo_transform_from_aabb(aabb: Aabb) -> Affine3A {
    let s = aabb.half_extents * 2.0;

    Affine3A::from_cols(
        Vec3A::new(s.x, 0.0, 0.0),
        Vec3A::new(0.0, s.y, 0.0),
        Vec3A::new(0.0, 0.0, s.z),
        aabb.center,
    )
}

// TODO: Avoid duplication from debug.rs.
fn gizmo_transform_from_aabb3d(aabb: Aabb3d) -> Affine3A {
    gizmo_transform_from_aabb(Aabb::from_min_max(aabb.min.into(), aabb.max.into()))
}

fn raycast(
    query: Query<(&SkinnedAabb, &SkinnedMesh)>,
    joints: Query<&GlobalTransform>,
    mut gizmos: Gizmos,
    assets: Res<Assets<SkinnedAabbAsset>>,
    ray_map: Res<RayMap>,
) {
    for (_, ray) in ray_map.iter() {
        query.iter().for_each(|(skinned_aabb, skinned_mesh)| {
            if let Some(asset) = assets.get(&skinned_aabb.asset) {
                for aabb_index in 0..asset.num_aabbs() {
                    if let Some(world_from_joint) =
                        asset.world_from_joint(aabb_index, skinned_mesh, &joints)
                        && ray_aabb_intersection_3d(
                            *ray,
                            &asset.aabb(aabb_index).into(),
                            &world_from_joint.into(),
                        )
                        .is_some()
                    {
                        let joint_from_aabb =
                            gizmo_transform_from_aabb3d(asset.aabb(aabb_index).into());
                        let world_from_aabb = world_from_joint * joint_from_aabb;

                        gizmos.cuboid(world_from_aabb, Color::WHITE);
                    }
                }
            }
        })
    }
}
