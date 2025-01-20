use bevy_asset::Assets;
use bevy_ecs::{prelude::*, system::FunctionSystem};
use bevy_mesh::{skinning::SkinnedMeshInverseBindposes, Mesh};
use bevy_mod_skinned_aabb::{
    create_skinned_aabbs,
    dev::{
        create_and_spawn_random_skinned_mesh, create_random_skinned_mesh_assets,
        spawn_random_skinned_mesh, RandomSkinnedMeshType,
    },
    update_skinned_aabbs_nonpar, SkinnedAabbAsset,
};
use bevy_tasks::{ComputeTaskPool, TaskPool};
use bevy_transform::prelude::*;
use core::time::Duration;
use criterion::{criterion_group, criterion_main, Bencher, Criterion};
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

#[derive(Resource, Copy, Clone)]
struct MeshParams {
    shared: bool,
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

    if params.shared {
        if let Ok(assets) = create_random_skinned_mesh_assets(
            &mut mesh_assets,
            &mut inverse_bindposes_assets,
            &mut rng,
            RandomSkinnedMeshType::Hard,
            1,
            params.num_joints,
        ) {
            let base = commands.spawn(Transform::IDENTITY).id();

            for _ in 0..params.num_meshes {
                spawn_random_skinned_mesh(
                    &mut commands,
                    &mut rng,
                    base,
                    Transform::IDENTITY,
                    &assets,
                );
            }
        }
    } else {
        for _ in 0..params.num_meshes {
            let _ = create_and_spawn_random_skinned_mesh(
                &mut commands,
                &mut mesh_assets,
                &mut inverse_bindposes_assets,
                &mut rng,
                base_entity,
                Transform::IDENTITY,
                RandomSkinnedMeshType::Hard,
                params.num_joints,
            );
        }
    }
}

fn bench_internal(b: &mut Bencher, mesh_params: &MeshParams) {
    ComputeTaskPool::get_or_init(TaskPool::default);

    let mut world = World::default();

    world.init_resource::<Assets<Mesh>>();
    world.init_resource::<Assets<SkinnedMeshInverseBindposes>>();
    world.init_resource::<Assets<SkinnedAabbAsset>>();
    world.insert_resource(*mesh_params);

    init_and_run_system(create_meshes, &mut world);
    init_and_run_system(create_skinned_aabbs, &mut world);

    let mut update_system = init_system(update_skinned_aabbs_nonpar, &mut world);
    b.iter(move || update_system.run((), &mut world));
}

pub fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("bench");
    group.warm_up_time(Duration::from_millis(100));
    group.measurement_time(Duration::from_secs(1));
    group.sample_size(10);

    struct Params {
        name: &'static str,
        mesh_params: MeshParams,
    }

    let params_list = [
        Params {
            name: "10000 joints total, 200 joints per mesh, shared asset",
            mesh_params: MeshParams {
                shared: true,
                num_meshes: 50,
                num_joints: 200,
            },
        },
        Params {
            name: "10000 joints total, 20 joints per mesh, shared asset",
            mesh_params: MeshParams {
                shared: true,
                num_meshes: 500,
                num_joints: 20,
            },
        },
        Params {
            name: "10000 joints total, 1 joint per mesh, shared asset",
            mesh_params: MeshParams {
                shared: true,
                num_meshes: 10000,
                num_joints: 1,
            },
        },
        Params {
            name: "10000 joints total, 200 joints per mesh, unique assets",
            mesh_params: MeshParams {
                shared: false,
                num_meshes: 50,
                num_joints: 200,
            },
        },
        Params {
            name: "10000 joints total, 20 joints per mesh, unique assets",
            mesh_params: MeshParams {
                shared: false,
                num_meshes: 500,
                num_joints: 20,
            },
        },
        Params {
            name: "10000 joints total, 1 joint per mesh, unique assets",
            mesh_params: MeshParams {
                shared: false,
                num_meshes: 10000,
                num_joints: 1,
            },
        },
    ];

    for params in params_list {
        group.bench_function(params.name, |b| bench_internal(b, &params.mesh_params));
    }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
