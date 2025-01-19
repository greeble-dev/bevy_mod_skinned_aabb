use bevy_asset::Assets;
use bevy_ecs::{prelude::*, system::FunctionSystem};
use bevy_mesh::{skinning::SkinnedMeshInverseBindposes, Mesh};
use bevy_mod_skinned_aabb::{
    create_skinned_aabbs, dev::spawn_random_mesh, update_skinned_aabbs_nonpar, SkinnedAabbAsset,
};
use bevy_render::primitives::Aabb;
use bevy_tasks::{ComputeTaskPool, TaskPool};
use bevy_transform::prelude::*;
use core::time::Duration;
use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

fn init_system<M, F>(func: F, world: &mut World) -> FunctionSystem<M, F>
where
    M: 'static,
    F: SystemParamFunction<M>,
{
    let mut system = IntoSystem::into_system(func);
    system.initialize(world);
    system.update_archetype_component_access(world.as_unsafe_world_cell());

    system
}

fn init_and_run_system<M, F>(func: F, world: &mut World)
where
    M: 'static,
    F: SystemParamFunction<M, In = ()>,
{
    init_system(func, world).run((), world);
}

const NUM_MESHES: usize = 100;
const NUM_TRIS: usize = 1000;
const NUM_JOINTS: usize = 100;
const MAX_INFLUENCES: usize = 4;

fn create_meshes(
    mut commands: Commands,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut inverse_bindposes_assets: ResMut<Assets<SkinnedMeshInverseBindposes>>,
) {
    let mut rng = ChaCha8Rng::seed_from_u64(732935);
    let base_entity = commands.spawn(Transform::IDENTITY).id();

    for _ in 0..NUM_MESHES {
        if let Ok(entity) = spawn_random_mesh(
            &mut rng,
            &mut commands,
            &mut mesh_assets,
            &mut inverse_bindposes_assets,
            base_entity,
            Transform::IDENTITY,
            NUM_TRIS,
            NUM_JOINTS,
            Some(MAX_INFLUENCES),
        ) {
            commands.entity(entity).insert(Aabb::default());
        }
    }
}

pub fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("bench");
    group.warm_up_time(Duration::from_millis(100));
    group.measurement_time(Duration::from_secs(1));
    group.throughput(Throughput::Elements((NUM_MESHES * NUM_JOINTS) as u64));
    group.bench_function("base", |b| {
        ComputeTaskPool::get_or_init(TaskPool::default);

        let mut world = World::default();

        world.init_resource::<Assets<Mesh>>();
        world.init_resource::<Assets<SkinnedMeshInverseBindposes>>();
        world.init_resource::<Assets<SkinnedAabbAsset>>();

        init_and_run_system(create_meshes, &mut world);
        init_and_run_system(create_skinned_aabbs, &mut world);

        let mut update_system = init_system(update_skinned_aabbs_nonpar, &mut world);
        b.iter(move || update_system.run((), &mut world));
    });
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
