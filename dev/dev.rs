// Utilities for tests and examples.

// TODO: Rust-analyzer complains about dead code even though all public
// functions are used in various tests and examples. Is there a better way to
// handle this?
#![allow(dead_code)]

use bevy::{
    hierarchy::BuildChildren,
    pbr::{MeshMaterial3d, StandardMaterial},
    tasks::{ComputeTaskPool, TaskPool},
    time::{Time, Virtual},
};
use bevy_asset::{Assets, Handle, RenderAssetUsages};
use bevy_color::Color;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    system::{Commands, Query, Res, ResMut},
    world::World,
};
use bevy_math::{
    curve::{Curve, EaseFunction, EasingCurve},
    ops, Affine3A, Mat4, Quat, Vec3,
};
use bevy_mesh::{
    skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
    Mesh, MeshVertexAttributeId, PrimitiveTopology, VertexAttributeValues,
};
use bevy_mod_skinned_aabb::{
    JointIndex, SkinnedAabbAsset, SkinnedAabbPluginSettings, MAX_INFLUENCES,
};
use bevy_render::{mesh::Mesh3d, primitives::Aabb, view::visibility::Visibility};
use bevy_transform::components::{GlobalTransform, Transform};
use rand::{
    distributions::{Distribution, Slice, Uniform},
    Rng, SeedableRng,
};
use rand_chacha::ChaCha8Rng;
use std::{borrow::Borrow, time::Duration};
use std::{
    f32::consts::TAU,
    hash::{DefaultHasher, Hash, Hasher},
};
use std::{iter::once, iter::repeat_with};

// Return a Vec3 with each element sampled from the given distribution.
fn random_vec3<T: Borrow<f32>, D: Distribution<T>>(rng: &mut impl Rng, dist: D) -> Vec3 {
    Vec3::new(
        *rng.sample(&dist).borrow(),
        *rng.sample(&dist).borrow(),
        *rng.sample(&dist).borrow(),
    )
}

// Return a Vec3 with each element uniformly sampled from the set (-1.0, 0.0, 1.0).
fn random_vec3_snorm_outlier(rng: &mut impl Rng) -> Vec3 {
    let dist = Slice::new(&[-1.0f32, 0.0f32, 1.0f32]).unwrap();

    random_vec3(rng, dist)
}

// Return a Vec3 with each element uniformly sampled from the range [-1.0, 1.0].
pub fn random_vec3_snorm(rng: &mut impl Rng) -> Vec3 {
    let dist = Uniform::new_inclusive(-1.0f32, 1.0f32);

    random_vec3(rng, dist)
}

// 50/50 chance of returning random_vec3_snorm or random_vec3_snorm_outlier.
fn random_vec3_snorm_maybe_outlier(rng: &mut impl Rng) -> Vec3 {
    if rng.gen::<bool>() {
        random_vec3_snorm(rng)
    } else {
        random_vec3_snorm_outlier(rng)
    }
}

// Return a random quaternion that's uniformly distributed on the 3-sphere.
//
// Source: Ken Shoemake, "Uniform Random Rotations", Graphics Gems III, Academic Press, 1992, pp. 124–132.
//
// We could have used Glam's default random instead. But it's implemented as a
// uniformly sampled axis and angle and so is not uniformly distributed on the
// 3-sphere. Which is probably fine for our purposes, but hey.
fn random_quat(rng: &mut impl Rng) -> Quat {
    let r0 = rng.gen_range(0.0f32..TAU);
    let r1 = rng.gen_range(0.0f32..TAU);
    let r2 = rng.gen_range(0.0f32..1.0f32);

    let (s0, c0) = ops::sin_cos(r0);
    let (s1, c1) = ops::sin_cos(r1);

    let t0 = (1.0 - r2).sqrt();
    let t1 = r2.sqrt();

    Quat::from_xyzw(t0 * s0, t0 * c0, t1 * s1, t1 * c1)
}

// Return a random quaternion that's identity or a 90/180 degree rotation
// around a single axis.
fn random_quat_outlier(rng: &mut impl Rng) -> Quat {
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

// 50/50 chance of returning random_quat or random_quat_outlier.
fn random_quat_maybe_outlier(rng: &mut impl Rng) -> Quat {
    if rng.gen::<bool>() {
        random_quat(rng)
    } else {
        random_quat_outlier(rng)
    }
}

fn random_transform(rng: &mut impl Rng) -> Transform {
    let translation = random_vec3_snorm(rng) * 0.5;
    let rotation: Quat = random_quat(rng);
    let scale = random_vec3_snorm(rng);

    Transform {
        translation,
        rotation,
        scale,
    }
}

fn random_transform_maybe_outlier(rng: &mut impl Rng) -> Transform {
    let translation = random_vec3_snorm_maybe_outlier(rng) * 0.5;
    let rotation: Quat = random_quat_maybe_outlier(rng);
    let scale = random_vec3_snorm_maybe_outlier(rng);

    Transform {
        translation,
        rotation,
        scale,
    }
}

pub enum RandomMeshError {
    InvalidNumJoints,
}

// Create a mesh with random triangles skinned to random joints with varying
// weights.
fn create_random_soft_skinned_mesh(
    rng: &mut impl Rng,
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

    let joint_indices = VertexAttributeValues::Uint16x4(joint_indices);

    Ok(Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_JOINT_INDEX, joint_indices)
    .with_inserted_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, joint_weights))
}

// Create a mesh with a triangle hard skinned to each joint.
fn create_random_hard_skinned_mesh(
    rng: &mut impl Rng,
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

    let joint_indices = VertexAttributeValues::Uint16x4(joint_indices);
    let joint_weights = vec![[1.0f32, 0.0f32, 0.0f32, 0.0f32]; num_verts];

    Ok(Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_JOINT_INDEX, joint_indices)
    .with_inserted_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, joint_weights))
}

fn create_random_inverse_bindposes(
    rng: &mut impl Rng,
    num_joints: usize,
) -> SkinnedMeshInverseBindposes {
    // Leaving the root as identity makes it more visually pleasing.
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
    rng: &mut impl Rng,
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

// An infinite timeline of discrete noise, with each sample one unit apart.
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
    rng: &mut impl Rng,
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
    rng: &mut impl Rng,
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
    rng: &mut impl Rng,
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

pub fn spawn_random_mesh_selection(
    mut commands: Commands,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
    mut inverse_bindposes_assets: ResMut<Assets<SkinnedMeshInverseBindposes>>,
) {
    let mut rng = ChaCha8Rng::seed_from_u64(732935);

    let material = MeshMaterial3d(material_assets.add(StandardMaterial {
        base_color: Color::WHITE,
        cull_mode: None,
        ..Default::default()
    }));

    struct MeshInstance {
        mesh_type: RandomSkinnedMeshType,
        num_joints: usize,
        translation: Vec3,
    }

    let mesh_instances = [
        MeshInstance {
            mesh_type: RandomSkinnedMeshType::Hard,
            num_joints: 1,
            translation: Vec3::new(-3.0, 1.5, 0.0),
        },
        MeshInstance {
            mesh_type: RandomSkinnedMeshType::Hard,
            num_joints: 20,
            translation: Vec3::new(0.0, 1.5, 0.0),
        },
        MeshInstance {
            mesh_type: RandomSkinnedMeshType::Hard,
            num_joints: 200,
            translation: Vec3::new(3.0, 1.5, 0.0),
        },
        MeshInstance {
            mesh_type: RandomSkinnedMeshType::Soft { num_tris: 100 },
            num_joints: 1,
            translation: Vec3::new(-3.0, -1.5, 0.0),
        },
        MeshInstance {
            mesh_type: RandomSkinnedMeshType::Soft { num_tris: 100 },
            num_joints: 20,
            translation: Vec3::new(0.0, -1.5, 0.0),
        },
        MeshInstance {
            mesh_type: RandomSkinnedMeshType::Soft { num_tris: 1000 },
            num_joints: 200,
            translation: Vec3::new(3.0, -1.5, 0.0),
        },
    ];

    for mesh_instance in mesh_instances {
        // Create a base entity. This will be the parent of the mesh and the joints.

        let base_transform = Transform::from_translation(mesh_instance.translation);
        let base_entity = commands.spawn((base_transform, Visibility::default())).id();

        // Give the mesh entity a random translation. This ensures we're not depending on the
        // mesh having the same transform as the root joint.

        let mesh_transform = Transform::from_translation(random_vec3_snorm(&mut rng));

        if let Ok(entity) = create_and_spawn_random_skinned_mesh(
            &mut commands,
            &mut mesh_assets,
            &mut inverse_bindposes_assets,
            &mut rng,
            base_entity,
            mesh_transform,
            mesh_instance.mesh_type,
            mesh_instance.num_joints,
        ) {
            commands.entity(entity).insert(material.clone());
        }
    }
}

pub fn update_random_mesh_animations(
    mut query: Query<(&mut Transform, &RandomMeshAnimation)>,
    time: Res<Time<Virtual>>,
) {
    for (mut transform, animation) in &mut query {
        // Sample the noise timeline and generate a transform for each key.

        let noise = animation.noise.sample(time.elapsed_secs());

        let t0 = random_transform_maybe_outlier(&mut ChaCha8Rng::seed_from_u64(noise.keys[0]));
        let t1 = random_transform_maybe_outlier(&mut ChaCha8Rng::seed_from_u64(noise.keys[1]));

        // Blend between the transforms with a nice ease in/out over 2/3rds of a
        // second, then hold for 1/3rd of a second.

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

// Create a `World` suitable for running our benchmarks and tests.
pub fn create_dev_world(settings: SkinnedAabbPluginSettings) -> World {
    ComputeTaskPool::get_or_init(TaskPool::default);

    let mut world = World::default();

    world.init_resource::<Assets<Mesh>>();
    world.init_resource::<Assets<SkinnedMeshInverseBindposes>>();
    world.init_resource::<Assets<SkinnedAabbAsset>>();
    world.init_resource::<Assets<StandardMaterial>>();

    world.insert_resource(settings);

    let mut time = Time::<Virtual>::default();
    time.advance_by(Duration::from_secs(1));

    world.insert_resource(time);

    world
}

pub enum SkinError {
    InvalidJointIndex,
    MismatchedJointAndInverseBindposesLengths,
    MismatchedMeshAttributeLengths,
    MissingJointEntity,
    MissingInverseBindposesAsset,
    MissingMeshAsset,
    UnexpectedPositionAttributeType,
    UnexpectedJointIndicesAttributeType,
    UnexpectedJointWeightsAttributeType,
}

fn skin_positions(
    positions: &VertexAttributeValues,
    joint_indices: &[[u16; 4]],
    joint_weights: &[[f32; 4]],
    entity_from_binds: &[Mat4],
) -> Result<Vec<[f32; 3]>, SkinError> {
    let VertexAttributeValues::Float32x3(positions) = positions else {
        return Err(SkinError::UnexpectedPositionAttributeType);
    };

    let mut out = vec![[0.0f32, 0.0f32, 0.0f32]; positions.len()];

    if joint_indices.len() != positions.len() {
        return Err(SkinError::MismatchedMeshAttributeLengths);
    }

    if joint_weights.len() != positions.len() {
        return Err(SkinError::MismatchedMeshAttributeLengths);
    }

    for (vertex_index, position) in positions.iter().enumerate() {
        let vertex_joint_indices = joint_indices[vertex_index];
        let vertex_joint_weights = joint_weights[vertex_index];

        let mut weighted_entity_from_binds = [Mat4::ZERO; 4];

        for influence_index in 0..4 {
            let joint_weight = vertex_joint_weights[influence_index];
            let joint_index = vertex_joint_indices[influence_index] as usize;
            let entity_from_bind = *entity_from_binds
                .get(joint_index)
                .ok_or(SkinError::InvalidJointIndex)?;

            weighted_entity_from_binds[influence_index] = joint_weight * entity_from_bind;
        }

        let entity_from_bind = weighted_entity_from_binds.iter().sum::<Mat4>();

        let skinned_position =
            <[f32; 3]>::from(entity_from_bind.transform_point3(Vec3::from_slice(position)));

        out[vertex_index] = skinned_position;
    }

    Ok(out)
}

fn skin_internal(
    mesh: &Mesh,
    inverse_bindposes: &[Mat4],
    entity_from_joints: &[Mat4],
) -> Result<Mesh, SkinError> {
    if entity_from_joints.len() != inverse_bindposes.len() {
        return Err(SkinError::MismatchedJointAndInverseBindposesLengths);
    }

    let Some(VertexAttributeValues::Uint16x4(joint_indices)) =
        mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX)
    else {
        return Err(SkinError::UnexpectedJointIndicesAttributeType);
    };

    let Some(VertexAttributeValues::Float32x4(joint_weights)) =
        mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT)
    else {
        return Err(SkinError::UnexpectedJointWeightsAttributeType);
    };

    let entity_from_binds = entity_from_joints
        .iter()
        .zip(inverse_bindposes.iter())
        .map(|(entity_from_joint, inverse_bindpose)| *entity_from_joint * *inverse_bindpose)
        .collect::<Vec<_>>();

    // TODO: Awkward? Appears needed since match patterns can't be expressions.
    const JOINT_INDEX_ID: MeshVertexAttributeId = Mesh::ATTRIBUTE_JOINT_INDEX.id;
    const JOINT_WEIGHT_ID: MeshVertexAttributeId = Mesh::ATTRIBUTE_JOINT_WEIGHT.id;
    const POSITION_ID: MeshVertexAttributeId = Mesh::ATTRIBUTE_POSITION.id;

    let mut out = Mesh::new(mesh.primitive_topology(), mesh.asset_usage);

    for (attribute, values) in mesh.attributes() {
        match attribute.id {
            JOINT_INDEX_ID => (),
            JOINT_WEIGHT_ID => (),

            POSITION_ID => {
                out.insert_attribute(
                    *attribute,
                    skin_positions(values, joint_indices, joint_weights, &entity_from_binds)?,
                );
            }

            _ => out.insert_attribute(*attribute, values.clone()),
        }
    }

    Ok(out)
}

fn try_entity_from_joint(
    joints: &Query<&GlobalTransform>,
    entity: Entity,
    entity_from_world: Affine3A,
) -> Option<Mat4> {
    let world_from_joint = joints.get(entity).ok()?.affine();

    Some(Mat4::from(entity_from_world * world_from_joint))
}

// Given the components of a skinned mesh, return a copy of the mesh with
// positions skinned to the current joint transforms. The mesh's skinning
// attributes are removed. Tangents and normals are *not* skinned.
pub fn skin(
    mesh: &Mesh3d,
    skinned_mesh: &SkinnedMesh,
    world_from_entity: &GlobalTransform,
    mesh_assets: &Assets<Mesh>,
    inverse_bindposes_assets: &Assets<SkinnedMeshInverseBindposes>,
    joint_transforms: &Query<&GlobalTransform>,
) -> Result<Mesh, SkinError> {
    let entity_from_world = world_from_entity.affine().inverse();

    let entity_from_joints = skinned_mesh
        .joints
        .iter()
        .map(|&entity| try_entity_from_joint(joint_transforms, entity, entity_from_world))
        .collect::<Option<Vec<_>>>()
        .ok_or(SkinError::MissingJointEntity)?;

    let mesh_asset = mesh_assets
        .get(&mesh.0)
        .ok_or(SkinError::MissingMeshAsset)?;

    let inverse_bindposes_asset = inverse_bindposes_assets
        .get(&skinned_mesh.inverse_bindposes)
        .ok_or(SkinError::MissingInverseBindposesAsset)?;

    skin_internal(mesh_asset, inverse_bindposes_asset, &entity_from_joints)
}
