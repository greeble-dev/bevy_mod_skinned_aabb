use bevy_asset::Assets;
use bevy_ecs::prelude::*;
use bevy_mesh::{skinning::SkinnedMeshInverseBindposes, Mesh};
use bevy_mod_skinned_aabb::{
    create_skinned_aabbs,
    dev::{
        create_dev_world, create_random_skinned_mesh_assets, init_and_run_system, init_system,
        spawn_random_skinned_mesh, RandomSkinnedMeshType,
    },
    update_skinned_aabbs, SkinnedAabbSettings,
};
use bevy_transform::prelude::*;
use core::time::Duration;
use criterion::{criterion_group, criterion_main, Bencher, Criterion, Throughput};
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

fn bench_internal(b: &mut Bencher, settings: SkinnedAabbSettings, mesh_params: &MeshParams) {
    let mut world = create_dev_world(settings);

    world.insert_resource(*mesh_params);

    init_and_run_system(create_meshes, &mut world);
    init_and_run_system(create_skinned_aabbs, &mut world);

    let mut update_system = init_system(update_skinned_aabbs, &mut world);
    b.iter(move || update_system.run((), &mut world));
}

pub fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("bench");

    group.warm_up_time(Duration::from_millis(100));
    group.measurement_time(Duration::from_millis(1000));

    for parallel in [false, true] {
        for num_assets in [1, 100] {
            for num_joints_total in [1_000, 10_000, 100_000, 1_000_000] {
                group.sample_size(if num_joints_total >= 100_000 { 10 } else { 50 });
                group.throughput(Throughput::Elements(num_joints_total as u64));

                for num_meshes in [10_000, 1_000, 100, 10] {
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

                    let settings = SkinnedAabbSettings { parallel };

                    group.bench_function(name, |b| bench_internal(b, settings, &mesh_params));
                }
            }
        }
    }

    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
