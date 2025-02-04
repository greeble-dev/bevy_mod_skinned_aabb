#[path = "../dev/dev.rs"]
mod dev;

use bevy_asset::Assets;
use bevy_ecs::{prelude::*, system::RunSystemOnce};
use bevy_mesh::{skinning::SkinnedMeshInverseBindposes, Mesh};
use bevy_mod_skinned_aabb::{
    create_skinned_aabbs, update_skinned_aabbs, SkinnedAabbPluginSettings,
};
use bevy_transform::prelude::*;
use core::time::Duration;
use criterion::{criterion_group, criterion_main, Bencher, Criterion, Throughput};
use dev::{
    create_dev_world, create_random_skinned_mesh_assets, spawn_random_skinned_mesh,
    RandomSkinnedMeshType,
};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::iter::repeat_with;

#[derive(Resource, Copy, Clone)]
struct MeshParams {
    num_assets: usize,
    num_meshes: usize,
    num_joints: usize,
}

fn create_meshes(
    mut commands: Commands,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut inverse_bindposes_assets: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    params: Res<MeshParams>,
) {
    let mut rng = ChaCha8Rng::seed_from_u64(732935);
    let base_entity = commands.spawn(Transform::IDENTITY).id();

    let assets = repeat_with(|| {
        create_random_skinned_mesh_assets(
            &mut mesh_assets,
            &mut inverse_bindposes_assets,
            &mut rng,
            RandomSkinnedMeshType::Hard,
            1,
            params.num_joints,
        )
        .ok()
    })
    .take(params.num_assets)
    .flatten()
    .collect::<Vec<_>>();

    for entity_index in 0..params.num_meshes {
        spawn_random_skinned_mesh(
            &mut commands,
            &mut rng,
            base_entity,
            Transform::IDENTITY,
            &assets[entity_index % assets.len()],
        );
    }
}

fn bench_internal(b: &mut Bencher, settings: SkinnedAabbPluginSettings, mesh_params: &MeshParams) {
    let world = &mut create_dev_world(settings);

    world.insert_resource(*mesh_params);

    world.run_system_once(create_meshes).unwrap();
    world.run_system_once(create_skinned_aabbs).unwrap();

    b.iter(move || world.run_system_cached(update_skinned_aabbs).unwrap());
}

pub fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("bench");

    group.warm_up_time(Duration::from_millis(100));
    group.measurement_time(Duration::from_millis(1000));

    struct Combo {
        num_joints_total: usize,
        num_meshes: usize,
    }

    let combos = [
        Combo {
            num_joints_total: 1_000,
            num_meshes: 100,
        },
        Combo {
            num_joints_total: 10_000,
            num_meshes: 100,
        },
        Combo {
            num_joints_total: 10_000,
            num_meshes: 1_000,
        },
        Combo {
            num_joints_total: 100_000,
            num_meshes: 1_000,
        },
        Combo {
            num_joints_total: 100_000,
            num_meshes: 10_000,
        },
        Combo {
            num_joints_total: 1_000_000,
            num_meshes: 10_000,
        },
    ];

    let num_assets = 10;

    for parallel in [false, true] {
        for &Combo {
            num_joints_total,
            num_meshes,
        } in &combos
        {
            group.warm_up_time(Duration::from_millis(500));

            if num_joints_total < 100_000 {
                group.sample_size(100);
                group.measurement_time(Duration::from_millis(500));
            } else {
                group.sample_size(50);
                group.measurement_time(Duration::from_millis(2000));
            }

            group.throughput(Throughput::Elements(num_joints_total as u64));

            if num_joints_total < num_meshes {
                continue;
            }

            assert!((num_joints_total % num_meshes) == 0);

            let num_joints = num_joints_total / num_meshes;

            // TODO: Correct constant?
            if num_joints >= 255 {
                continue;
            }

            let name = format!(
                        "(parallel = {}, assets = {}, joints total = {}, joints per mesh = {}, meshes = {})",
                        parallel, num_assets, num_joints_total, num_joints, num_meshes,
                    );

            let mesh_params = MeshParams {
                num_assets,
                num_meshes,
                num_joints,
            };

            let settings = SkinnedAabbPluginSettings { parallel };

            group.bench_function(name, |b| bench_internal(b, settings, &mesh_params));
        }
    }

    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
