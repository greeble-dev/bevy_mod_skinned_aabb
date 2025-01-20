// Utilities for tests and examples.
//
// This module is public so that it can be used by the crate's tests and
// examples. It's not intended to be used by anything outside the crate.

use crate::{JointIndex, MAX_INFLUENCES};
use bevy_asset::{Assets, Handle, RenderAssetUsages};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    system::{Commands, Query, Res},
};
use bevy_hierarchy::BuildChildren;
use bevy_math::{
    curve::{Curve, EaseFunction, EasingCurve},
    ops, Mat4, Quat, Vec3,
};
use bevy_mesh::Mesh;
use bevy_render::{
    mesh::{
        skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
        Mesh3d, PrimitiveTopology, VertexAttributeValues,
    },
    primitives::Aabb,
};
use bevy_time::{Time, Virtual};
use bevy_transform::components::Transform;
use rand::{
    distributions::{Distribution, Slice, Uniform},
    Rng, RngCore, SeedableRng,
};
use rand_chacha::ChaCha8Rng;
use std::borrow::Borrow;
use std::{
    f32::consts::TAU,
    hash::{DefaultHasher, Hash, Hasher},
};
use std::{iter::once, iter::repeat_with};

// Returns a Vec3 with each element sampled from the given distribution.
fn random_vec3<T: Borrow<f32>, D: Distribution<T>>(rng: &mut dyn RngCore, dist: D) -> Vec3 {
    Vec3::new(
        *rng.sample(&dist).borrow(),
        *rng.sample(&dist).borrow(),
        *rng.sample(&dist).borrow(),
    )
}

// Returns a Vec3 with each element uniformly sampled from the set [-1.0, 0.0, 1.0].
fn random_outlier_vec3_snorm(rng: &mut dyn RngCore) -> Vec3 {
    let dist = Slice::new(&[-1.0f32, 0.0f32, 1.0f32]).unwrap();

    random_vec3(rng, dist)
}

// Returns a Vec3 with each element uniformly sampled from the range [-1.0, 1.0].
pub fn random_vec3_snorm(rng: &mut dyn RngCore) -> Vec3 {
    let dist = Uniform::new_inclusive(-1.0f32, 1.0f32);

    random_vec3(rng, dist)
}

// 50/50 chance of returning random_vec3_snorm or random_outlier_vec3_snorm.
fn random_maybe_outlier_vec3_snorm(rng: &mut dyn RngCore) -> Vec3 {
    if rng.gen::<bool>() {
        random_vec3_snorm(rng)
    } else {
        random_outlier_vec3_snorm(rng)
    }
}

// Returns a random quaternion that's uniformly distributed on the 3-sphere.
//
// Source: Ken Shoemake, "Uniform Random Rotations", Graphics Gems III, Academic Press, 1992, pp. 124–132.
//
// We could have used Glam's default random instead. But it's implemented as a uniformly sampled axis and
// angle and so is not uniformly distributed on the 3-sphere. Which is probably fine for our purposes, but hey.
fn random_quat(rng: &mut dyn RngCore) -> Quat {
    // TODO: Should these ranges be inclusive or not?
    let r0 = rng.gen_range(0.0f32..TAU);
    let r1 = rng.gen_range(0.0f32..TAU);
    let r2 = rng.gen_range(0.0f32..1.0f32);

    let (s0, c0) = ops::sin_cos(r0);
    let (s1, c1) = ops::sin_cos(r1);

    let t0 = (1.0 - r2).sqrt();
    let t1 = r2.sqrt();

    Quat::from_xyzw(t0 * s0, t0 * c0, t1 * s1, t1 * c1)
}

fn random_outlier_quat(rng: &mut dyn RngCore) -> Quat {
    let a90 = 1.0 / 2.0f32.sqrt();

    let values = [
        Quat::from_xyzw(1.0, 0.0, 0.0, 0.0),
        Quat::from_xyzw(0.0, 1.0, 0.0, 0.0),
        Quat::from_xyzw(0.0, 0.0, 1.0, 0.0),
        Quat::from_xyzw(0.0, 0.0, 0.0, 1.0),
        Quat::from_xyzw(a90, a90, 0.0, 0.0),
        Quat::from_xyzw(a90, 0.0, a90, 0.0),
        Quat::from_xyzw(a90, 0.0, 0.0, a90),
        Quat::from_xyzw(0.0, a90, a90, 0.0),
        Quat::from_xyzw(0.0, a90, 0.0, a90),
        Quat::from_xyzw(0.0, 0.0, a90, a90),
    ];

    *rng.sample(Slice::new(&values).unwrap())
}

// 50/50 chance of returning random_quat or random_outlier_quat.
fn random_maybe_outlier_quat(rng: &mut dyn RngCore) -> Quat {
    if rng.gen::<bool>() {
        random_quat(rng)
    } else {
        random_outlier_quat(rng)
    }
}

fn random_transform(rng: &mut dyn RngCore) -> Transform {
    let translation = random_maybe_outlier_vec3_snorm(rng) * 0.5;
    let rotation: Quat = random_maybe_outlier_quat(rng);
    let scale = random_maybe_outlier_vec3_snorm(rng);

    Transform {
        translation,
        rotation,
        scale,
    }
}

pub enum RandomMeshError {
    InvalidNumJoints,
}

fn create_random_soft_skinned_mesh(
    rng: &mut dyn RngCore,
    num_tris: usize,
    num_unskinned_joints: usize,
    num_skinned_joints: usize,
) -> Result<Mesh, RandomMeshError> {
    let num_joints = JointIndex::try_from(num_unskinned_joints + num_skinned_joints)
        .or(Err(RandomMeshError::InvalidNumJoints))?;

    let position_dist = Uniform::new_inclusive(-0.5, 0.5);
    let joint_index_dist = Uniform::new(num_unskinned_joints as JointIndex, num_joints);
    let joint_weight_dist = Uniform::new(0.01, 1.0);
    let num_influences_dist = Uniform::new_inclusive(1, MAX_INFLUENCES);

    let num_verts = num_tris * 3;

    let mut positions = vec![Vec3::ZERO; num_verts];
    let mut joint_indices = vec![[0u16; 4]; num_verts];
    let mut joint_weights = vec![[0.0f32; 4]; num_verts];

    for vert_index in 0..num_verts {
        let position = random_vec3(rng, position_dist);

        let mut vert_joint_indices = [0u16; MAX_INFLUENCES];
        let mut vert_joint_weights = [0.0f32; MAX_INFLUENCES];

        for influence_index in 0..rng.sample(num_influences_dist) {
            vert_joint_indices[influence_index] = rng.sample(joint_index_dist);
            vert_joint_weights[influence_index] = rng.sample(joint_weight_dist);
        }

        let normalization_scale = 1.0 / vert_joint_weights.iter().sum::<f32>();
        let vert_joint_weights = vert_joint_weights.map(|w| w * normalization_scale);

        positions[vert_index] = position;
        joint_indices[vert_index] = vert_joint_indices;
        joint_weights[vert_index] = vert_joint_weights;
    }

    Ok(Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_JOINT_INDEX,
        VertexAttributeValues::Uint16x4(joint_indices),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, joint_weights))
}

fn create_random_hard_skinned_mesh(
    rng: &mut dyn RngCore,
    num_unskinned_joints: usize,
    num_skinned_joints: usize,
) -> Result<Mesh, RandomMeshError> {
    // Check that all the joints can fit in a JointIndex.
    if JointIndex::try_from(num_unskinned_joints + num_skinned_joints).is_err() {
        return Err(RandomMeshError::InvalidNumJoints);
    };

    let position_dist = Uniform::new_inclusive(-0.5, 0.5);

    let num_tris = num_skinned_joints;
    let num_verts = num_tris * 3;

    let mut positions = vec![Vec3::ZERO; num_verts];
    let mut joint_indices = vec![[0u16; 4]; num_verts];

    // More tris = smaller tris.
    let scale = 1.0 / ((num_skinned_joints as f32) * 0.2).cbrt();

    for tri_index in 0..num_skinned_joints {
        let joint_index = (num_unskinned_joints + tri_index) as JointIndex;

        let base_position = random_vec3(rng, position_dist);

        let tri_vert_positions = [
            base_position + (scale * random_vec3(rng, position_dist)),
            base_position + (scale * random_vec3(rng, position_dist)),
            base_position + (scale * random_vec3(rng, position_dist)),
        ];

        let vert_joint_indices = [joint_index, 0, 0, 0];

        for (tri_vert_index, tri_vert_position) in tri_vert_positions.iter().enumerate() {
            let vert_index = (tri_index * 3) + tri_vert_index;
            positions[vert_index] = *tri_vert_position;
            joint_indices[vert_index] = vert_joint_indices;
        }
    }

    let joint_weights = vec![[1.0f32, 0.0f32, 0.0f32, 0.0f32]; num_verts];

    Ok(Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_JOINT_INDEX,
        VertexAttributeValues::Uint16x4(joint_indices),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, joint_weights))
}

fn create_random_inverse_bindposes(
    rng: &mut dyn RngCore,
    num_joints: usize,
) -> SkinnedMeshInverseBindposes {
    // Leave the root as identity so it's more visually pleasing.
    let iter = once(Mat4::IDENTITY).chain(repeat_with(|| random_transform(rng).compute_matrix()));

    SkinnedMeshInverseBindposes::from(iter.take(num_joints).collect::<Vec<_>>())
}

pub struct SkinnedMeshAssets {
    mesh: Handle<Mesh>,
    inverse_bindposes: Handle<SkinnedMeshInverseBindposes>,
    num_joints: usize,
}

pub enum RandomSkinnedMeshType {
    Hard,
    Soft { num_tris: usize },
}

pub fn create_random_skinned_mesh_assets(
    mesh_assets: &mut Assets<Mesh>,
    inverse_bindposes_assets: &mut Assets<SkinnedMeshInverseBindposes>,
    rng: &mut dyn RngCore,
    mesh_type: RandomSkinnedMeshType,
    num_unskinned_joints: usize,
    num_skinned_joints: usize,
) -> Result<SkinnedMeshAssets, RandomMeshError> {
    let num_joints = num_unskinned_joints + num_skinned_joints;

    let mesh = match mesh_type {
        RandomSkinnedMeshType::Soft { num_tris } => {
            create_random_soft_skinned_mesh(rng, num_tris, num_unskinned_joints, num_skinned_joints)
        }
        RandomSkinnedMeshType::Hard => {
            create_random_hard_skinned_mesh(rng, num_unskinned_joints, num_skinned_joints)
        }
    }?;

    let mesh = mesh_assets.add(mesh);

    let inverse_bindposes =
        inverse_bindposes_assets.add(create_random_inverse_bindposes(rng, num_joints));

    Ok(SkinnedMeshAssets {
        mesh,
        inverse_bindposes,
        num_joints,
    })
}

// Hash a single value.
fn hash<T: Hash>(v: T) -> u64 {
    let mut hasher = DefaultHasher::new();
    v.hash(&mut hasher);
    hasher.finish()
}

// An infinite timeline of noise, where each item of noise is one unit of time apart.
struct NoiseTimeline {
    seed: u64,
}

// A sample of a NoiseTimeline.
struct NoiseSample {
    // The two noise values before and after the sample time.
    keys: [u64; 2],

    // The alpha of the time between the noise values. 0.0 == keys[0], 1.0 == keys[1].
    alpha: f32,
}

impl NoiseTimeline {
    fn sample(&self, time: f32) -> NoiseSample {
        assert!(time >= 0.0);

        let alpha = time.fract();
        let basis = self.seed.wrapping_add(time.trunc() as u64);
        let keys = [hash(basis), hash(basis.wrapping_add(1))];

        NoiseSample { keys, alpha }
    }
}

#[derive(Component)]
pub struct RandomMeshAnimation {
    noise: NoiseTimeline,
}

impl RandomMeshAnimation {
    fn new(seed: u64) -> Self {
        RandomMeshAnimation {
            noise: NoiseTimeline { seed },
        }
    }
}

pub fn spawn_joints(
    commands: &mut Commands,
    rng: &mut dyn RngCore,
    base: Entity,
    num: usize,
) -> Vec<Entity> {
    assert!(num > 0);

    let mut joints: Vec<Entity> = Vec::with_capacity(num);

    let root_joint = commands
        .spawn((Transform::IDENTITY, RandomMeshAnimation::new(rng.gen())))
        .set_parent(base)
        .id();

    joints.push(root_joint);

    for _ in 1..num {
        let joint = commands
            .spawn((Transform::IDENTITY, RandomMeshAnimation::new(rng.gen())))
            .set_parent(root_joint)
            .id();

        joints.push(joint);
    }

    joints
}

pub fn spawn_random_skinned_mesh(
    commands: &mut Commands,
    rng: &mut dyn RngCore,
    base: Entity,
    transform: Transform,
    assets: &SkinnedMeshAssets,
) -> Entity {
    let joints = spawn_joints(commands, rng, base, assets.num_joints);

    commands
        .spawn((
            transform,
            Mesh3d(assets.mesh.clone()),
            SkinnedMesh {
                inverse_bindposes: assets.inverse_bindposes.clone(),
                joints,
            },
            Aabb::default(),
        ))
        .set_parent(base)
        .id()
}

#[allow(clippy::too_many_arguments)]
pub fn create_and_spawn_random_skinned_mesh(
    commands: &mut Commands,
    mesh_assets: &mut Assets<Mesh>,
    inverse_bindposes_assets: &mut Assets<SkinnedMeshInverseBindposes>,
    rng: &mut dyn RngCore,
    base: Entity,
    transform: Transform,
    mesh_type: RandomSkinnedMeshType,
    num_skinned_joints: usize,
) -> Result<Entity, RandomMeshError> {
    let num_unskinned_joints = 1;

    let assets = create_random_skinned_mesh_assets(
        mesh_assets,
        inverse_bindposes_assets,
        rng,
        mesh_type,
        num_unskinned_joints,
        num_skinned_joints,
    )?;

    Ok(spawn_random_skinned_mesh(
        commands, rng, base, transform, &assets,
    ))
}

pub fn update_random_meshes(
    mut query: Query<(&mut Transform, &RandomMeshAnimation)>,
    time: Res<Time<Virtual>>,
) {
    for (mut transform, animation) in &mut query {
        let noise = animation.noise.sample(time.elapsed_secs());

        let t0 = random_transform(&mut ChaCha8Rng::seed_from_u64(noise.keys[0]));
        let t1 = random_transform(&mut ChaCha8Rng::seed_from_u64(noise.keys[1]));

        // Blend between transforms with a nice ease in/out, and hold each transform
        // for 1/3rd of a second.

        let ease = EasingCurve::new(0.0, 1.0, EaseFunction::CubicInOut);
        let alpha = ease.sample_clamped(noise.alpha * 1.5);

        // TODO: Feels like there should be a standard function for mixing two transforms?

        *transform = Transform {
            translation: t0.translation.lerp(t1.translation, alpha),
            rotation: t0.rotation.lerp(t1.rotation, alpha),
            scale: t0.scale.lerp(t1.scale, alpha),
        };
    }
}
