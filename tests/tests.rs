use bevy::prelude::*;
use bevy_mod_skinned_aabb::{
    create_skinned_aabbs,
    dev::{
        create_dev_world, init_and_run_system, init_system, spawn_random_mesh_selection,
        update_random_mesh_animations,
    },
    update_skinned_aabbs, SkinnedAabbSettings,
};

#[test]
fn test() {
    let mut world = create_dev_world(SkinnedAabbSettings::default());

    init_and_run_system(spawn_random_mesh_selection, &mut world);
    init_and_run_system(create_skinned_aabbs, &mut world);

    let mut update_system = init_system(update_skinned_aabbs, &mut world);
    let mut animation_system = init_system(update_random_mesh_animations, &mut world);

    for _ in 0..10 {
        update_system.run((), &mut world);
        animation_system.run((), &mut world);
    }
}
