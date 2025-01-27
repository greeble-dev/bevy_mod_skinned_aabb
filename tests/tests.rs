use bevy::prelude::*;
use bevy_ecs::system::RunSystemOnce;
use bevy_math::Vec3A;
use bevy_mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy_mod_skinned_aabb::{
    create_skinned_aabbs,
    dev::{create_dev_world, skin, spawn_random_mesh_selection, update_random_mesh_animations},
    update_skinned_aabbs, SkinnedAabbSettings,
};
use bevy_render::{mesh::MeshAabb, primitives::Aabb};

fn test_against_cpu_skinning(
    query: Query<(&Mesh3d, &SkinnedMesh, &GlobalTransform, &Aabb)>,
    joints: Query<&GlobalTransform>,
    inverse_bindposes_assets: Res<Assets<SkinnedMeshInverseBindposes>>,
    mesh_assets: Res<Assets<Mesh>>,
) {
    assert!(
        query.iter().count() > 0,
        "Missing expected components or entities."
    );

    for (mesh, skinned_mesh, transform, aabb) in query.iter() {
        if let Ok(cpu_skinned_mesh) = skin(
            mesh,
            skinned_mesh,
            transform,
            &mesh_assets,
            &inverse_bindposes_assets,
            &joints,
        ) {
            if let Some(cpu_skinned_aabb) = cpu_skinned_mesh.compute_aabb() {
                // The accurate AABB calculated from the skinned vertices should
                // always be contained within our conservative AABB calculated
                // from the joint AABBs.

                let accurate_min = cpu_skinned_aabb.min();
                let accurate_max = cpu_skinned_aabb.max();

                let conservative_min = aabb.min();
                let conservative_max = aabb.max();

                let epsilon = Vec3A::splat(0.001);

                assert!(
                    conservative_min.cmplt(accurate_min + epsilon).all(),
                    "Conservative minimum {conservative_min} should not be greater than the accurate minimum {accurate_min}.",
                );
                assert!(
                    conservative_max.cmpge(accurate_max - epsilon).all(),
                    "Conservative maximum {conservative_max} should not be less than the accurate maximum {accurate_max}.",
                );
            } else {
                unreachable!("Failed to compute AABB.");
            }
        } else {
            unreachable!("Failed to skin mesh.");
        }
    }
}

#[test]
fn test() {
    let world = &mut create_dev_world(SkinnedAabbSettings::default());

    world.run_system_once(spawn_random_mesh_selection).unwrap();
    world.run_system_once(create_skinned_aabbs).unwrap();

    for _ in 0..100 {
        world.run_system_cached(update_skinned_aabbs).unwrap();
        world
            .run_system_cached(update_random_mesh_animations)
            .unwrap();
        world.run_system_cached(test_against_cpu_skinning).unwrap();
    }
}
