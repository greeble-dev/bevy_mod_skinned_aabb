use bevy::prelude::*;
use bevy_mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy_mod_skinned_aabb::dev::{
    skin, spawn_random_mesh_selection, update_random_mesh_animations,
};
use bevy_render::{camera::ScalingMode, primitives::Aabb};

fn main() {
    App::new()
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 2000.,
        })
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Startup, spawn_random_mesh_selection)
        .add_systems(Update, update_random_mesh_animations)
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
            cpu_skinning_spawn_new
                .after(cpu_skinning_delete_existing)
                .before(update_random_mesh_animations),
        )
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: 16.0 * 1.1,
                min_height: 9.0 * 1.1,
            },
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_xyz(4.0, 0.0, 12.0).looking_at(Vec3::new(4.0, 0.0, 0.0), Vec3::Y),
    ));
}

#[derive(Component, Default)]
struct CpuSkinningMarker;

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
    for (mesh, skinned_mesh, transform, aabb, material) in query.iter() {
        let Ok(cpu_skinned_mesh) = skin(
            mesh,
            skinned_mesh,
            transform,
            &mesh_assets,
            &inverse_bindposes_assets,
            &joints,
        ) else {
            continue;
        };

        let cpu_skinned_transform = Transform::from_xyz(8.0, 0.0, 0.0) * *transform;
        let cpu_skinned_mesh_asset = Mesh3d(mesh_assets.add(cpu_skinned_mesh));

        commands.spawn((
            cpu_skinned_mesh_asset,
            material.clone(),
            *aabb,
            Transform::from(cpu_skinned_transform),
            CpuSkinningMarker,
        ));
    }
}
