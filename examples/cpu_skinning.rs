use bevy::{prelude::*, scene::SceneInstanceReady};
use bevy_mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy_mod_skinned_aabb::dev::skin;
use bevy_render::primitives::Aabb;

const GLTF_PATH: &str = "Fox.glb";

fn main() {
    App::new()
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 2000.,
        })
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup_mesh_and_animation)
        .add_systems(Startup, setup_camera_and_environment)
        .add_systems(Update, cpu_skinning_delete_existing)
        /*
        // TODO: Why doesn't this work? Would avoid us being a frame behind.
        // Probably missing some required components but not sure what... tried
        // GlobalTransform, Visibility::Visible, ViewVisibility.
        .add_systems(
            PostUpdate,
            cpu_skinning_spawn_new.after(TransformSystem::TransformPropagate),
        )
        */
        .add_systems(
            Update,
            cpu_skinning_spawn_new.after(cpu_skinning_delete_existing),
        )
        .run();
}

#[derive(Component)]
struct AnimationToPlay {
    graph_handle: Handle<AnimationGraph>,
    index: AnimationNodeIndex,
}

#[derive(Component, Default)]
struct CpuSkinningMarker;

fn setup_mesh_and_animation(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let (graph, index) = AnimationGraph::from_clip(
        asset_server.load(GltfAssetLabel::Animation(2).from_asset(GLTF_PATH)),
    );

    let graph_handle = graphs.add(graph);

    let animation_to_play = AnimationToPlay {
        graph_handle,
        index,
    };

    let mesh_scene = SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(GLTF_PATH)));

    commands
        .spawn((
            animation_to_play,
            mesh_scene,
            Transform::from_xyz(-30.0, 0.0, 0.0),
        ))
        .observe(play_animation_when_ready);
}

fn play_animation_when_ready(
    trigger: Trigger<SceneInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    animations_to_play: Query<&AnimationToPlay>,
    mut players: Query<&mut AnimationPlayer>,
) {
    if let Ok(animation_to_play) = animations_to_play.get(trigger.entity()) {
        for child in children.iter_descendants(trigger.entity()) {
            if let Ok(mut player) = players.get_mut(child) {
                player.play(animation_to_play.index).repeat();

                commands
                    .entity(child)
                    .insert(AnimationGraphHandle(animation_to_play.graph_handle.clone()));
            }
        }
    }
}

fn cpu_skinning_delete_existing(
    mut commands: Commands,
    query: Query<(Entity, &CpuSkinningMarker)>,
) {
    for (entity, _) in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn cpu_skinning_spawn_new(
    mut commands: Commands,
    query: Query<(
        &Mesh3d,
        &SkinnedMesh,
        &GlobalTransform,
        &Aabb,
        &MeshMaterial3d<StandardMaterial>,
    )>,
    joints: Query<&GlobalTransform>,
    inverse_bindposes_assets: Res<Assets<SkinnedMeshInverseBindposes>>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
) {
    let cpu_skinned_transform = Transform::from_xyz(30.0, 0.0, 0.0);

    for (mesh, skinned_mesh, transform, aabb, material) in query.iter() {
        let Ok(cpu_skinned_mesh) = skin(
            mesh,
            skinned_mesh,
            &transform.affine(),
            &mesh_assets,
            &inverse_bindposes_assets,
            &joints,
        ) else {
            continue;
        };

        let cpu_skinned_mesh_asset = Mesh3d(mesh_assets.add(cpu_skinned_mesh));

        commands.spawn((
            cpu_skinned_mesh_asset,
            material.clone(),
            *aabb,
            cpu_skinned_transform,
            CpuSkinningMarker,
        ));
    }
}

fn setup_camera_and_environment(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 100.0, 150.0).looking_at(Vec3::new(0.0, 30.0, 0.0), Vec3::Y),
    ));
}
