#[path = "../dev/dev.rs"]
mod dev;

use bevy_asset::Assets;
use bevy_ecs::{prelude::*, system::RunSystemOnce};
use bevy_math::{
    Affine3A, Vec3, Vec3A,
    bounding::{Aabb3d, BoundingVolume},
};
use bevy_mesh::{Mesh, skinning::SkinnedMeshInverseBindposes};
use bevy_mod_skinned_aabb::{
    PackedAabb3d, SkinnedAabbPluginSettings, aabb_transformed_by, create_skinned_aabbs,
    update_skinned_aabbs,
};
use bevy_transform::prelude::*;
use core::time::Duration;
use criterion::{Bencher, Criterion, Throughput, black_box, criterion_group, criterion_main};
use dev::{
    RandomSkinnedMeshType, create_dev_world, create_random_skinned_mesh_assets,
    spawn_random_skinned_mesh,
};
use rand::{SeedableRng, rngs::StdRng};
use std::iter::repeat_with;

#[derive(Resource, Copy, Clone)]
struct MeshParams {
    num_assets: usize,
    num_meshes: usize,
    num_joints: usize,
}

pub fn core_data() -> (usize, Vec<PackedAabb3d>, Vec<Affine3A>) {
    let count: usize =
        black_box((128 * 1024) / (size_of::<PackedAabb3d>() + size_of::<Affine3A>()));

    let aabbs = vec![
        PackedAabb3d {
            min: Vec3::ZERO,
            max: Vec3::ZERO,
        };
        count
    ];

    let joints = vec![Affine3A::IDENTITY; count];

    (count, aabbs, joints)
}

#[inline(never)]
fn core_basic_fold_inner(aabbs: &[PackedAabb3d], joints: &[Affine3A]) -> Aabb3d {
    let count = aabbs.len().min(joints.len());

    if count == 0 {
        panic!()
    }

    let mut t = Aabb3d {
        min: Vec3A::MAX,
        max: Vec3A::MIN,
    };

    for index in 0..count {
        t = t.merge(&aabb_transformed_by(aabbs[index], joints[index]));
    }

    t
}

#[inline(never)]
fn core_basic_reduce_inner(aabbs: &[PackedAabb3d], joints: &[Affine3A]) -> Aabb3d {
    let count = aabbs.len().min(joints.len());

    if count == 0 {
        panic!()
    }

    let mut t = aabb_transformed_by(aabbs[0], joints[0]);

    for index in 1..count {
        t = t.merge(&aabb_transformed_by(aabbs[index], joints[index]));
    }

    t
}

pub fn core_basic(c: &mut Criterion) {
    let mut group = c.benchmark_group("core_basic");

    let (count, aabbs, joints) = core_data();

    group.throughput(Throughput::Elements(count as u64));

    group.bench_function(format!("basic fold, count = {count}"), |b| {
        b.iter(|| black_box(core_basic_fold_inner(&aabbs, &joints)))
    });

    group.bench_function(format!("basic reduce, count = {count}"), |b| {
        b.iter(|| black_box(core_basic_reduce_inner(&aabbs, &joints)))
    });
}

#[inline(never)]
fn core_fancy_fold_inner(aabbs: &[PackedAabb3d], joints: &[Affine3A]) -> Aabb3d {
    let count = aabbs.len().min(joints.len());

    if count == 0 {
        panic!()
    }

    let initial = Aabb3d {
        min: Vec3A::MAX,
        max: Vec3A::MIN,
    };

    aabbs
        .iter()
        .zip(joints.iter())
        .map(|(aabb, joint)| aabb_transformed_by(*aabb, *joint))
        .fold(initial, |l, r| l.merge(&r))
}

#[inline(never)]
fn core_fancy_reduce_inner(aabbs: &[PackedAabb3d], joints: &[Affine3A]) -> Aabb3d {
    aabbs
        .iter()
        .zip(joints.iter())
        .map(|(aabb, joint)| aabb_transformed_by(*aabb, *joint))
        .reduce(|l, r| l.merge(&r))
        .unwrap()
}

pub fn core_fancy(c: &mut Criterion) {
    let mut group = c.benchmark_group("core_fancy");

    let (count, aabbs, joints) = core_data();

    group.throughput(Throughput::Elements(count as u64));

    group.bench_function(format!("fancy fold, count = {count}"), |b| {
        b.iter(|| black_box(core_fancy_fold_inner(&aabbs, &joints)))
    });

    group.bench_function(format!("fancy reduce, count = {count}"), |b| {
        b.iter(|| black_box(core_fancy_reduce_inner(&aabbs, &joints)))
    });
}

fn create_meshes(
    mut commands: Commands,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut inverse_bindposes_assets: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    params: Res<MeshParams>,
) {
    let mut rng = StdRng::seed_from_u64(732935);
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

fn systems_internal(
    b: &mut Bencher,
    settings: SkinnedAabbPluginSettings,
    mesh_params: &MeshParams,
) {
    let world = &mut create_dev_world(settings);

    world.insert_resource(*mesh_params);

    world.run_system_once(create_meshes).unwrap();
    world.run_system_once(create_skinned_aabbs).unwrap();

    b.iter(move || world.run_system_cached(update_skinned_aabbs).unwrap());
}

pub fn systems(c: &mut Criterion) {
    let mut group = c.benchmark_group("systems");

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

            group.bench_function(name, |b| systems_internal(b, settings, &mesh_params));
        }
    }

    group.finish();
}

criterion_group!(benches, core_basic, core_fancy, systems);
criterion_main!(benches);
