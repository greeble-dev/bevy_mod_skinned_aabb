use bevy::{
    asset::RenderAssetUsages,
    input::common_conditions::input_just_pressed,
    prelude::*,
    render::mesh::{
        skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
        PrimitiveTopology, VertexAttributeValues,
    },
};
use bevy_mod_skinned_aabb::{
    debug::{toggle_draw_joint_aabbs, toggle_draw_mesh_aabbs, SkinnedAabbDebugPlugin},
    JointIndex, SkinnedAabbPlugin, MAX_INFLUENCES,
};
use rand::{distributions::Slice, distributions::Uniform, prelude::Distribution, Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::{borrow::Borrow, iter::once, iter::repeat_with};
use std::{
    f32::consts::TAU,
    hash::{DefaultHasher, Hash, Hasher},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Skinned AABB Random Meshes".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(SkinnedAabbPlugin)
        .add_plugins(SkinnedAabbDebugPlugin::enable_by_default())
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 2000.0,
        })
        .add_systems(
            Update,
            (
                toggle_pause.run_if(input_just_pressed(KeyCode::Space)),
                toggle_draw_joint_aabbs.run_if(input_just_pressed(KeyCode::KeyJ)),
                toggle_draw_mesh_aabbs.run_if(input_just_pressed(KeyCode::KeyM)),
            ),
        )
        .add_systems(Startup, setup)
        .add_systems(Startup, spawn_random_meshes)
        .add_systems(Update, update_random_meshes)
        .run();
}

fn toggle_pause(mut time: ResMut<Time<Virtual>>) {
    if time.is_paused() {
        time.unpause();
    } else {
        time.pause();
    }
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 1.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Text::new("J: Toggle Joint AABBs\nM: Toggle Mesh AABBs"),
        TextFont {
            font_size: 15.0,
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

// Returns a Vec3 with each element sampled from the given distribution.
fn random_vec3<R: Rng + ?Sized, T: Borrow<f32>, D: Distribution<T>>(rng: &mut R, dist: D) -> Vec3 {
    Vec3::new(
        *rng.sample(&dist).borrow(),
        *rng.sample(&dist).borrow(),
        *rng.sample(&dist).borrow(),
    )
}

// Returns a Vec3 with each element uniformly sampled from the set [-1.0, 0.0, 1.0].
fn random_outlier_vec3_snorm<R: Rng + ?Sized>(rng: &mut R) -> Vec3 {
    let dist = Slice::new(&[-1.0f32, 0.0f32, 1.0f32]).unwrap();

    random_vec3(rng, dist)
}

// Returns a Vec3 with each element uniformly sampled from the range [-1.0, 1.0].
fn random_vec3_snorm<R: Rng + ?Sized>(rng: &mut R) -> Vec3 {
    let dist = Uniform::new_inclusive(-1.0f32, 1.0f32);

    random_vec3(rng, dist)
}

// 50/50 chance of returning random_vec3_snorm or random_outlier_vec3_snorm.
fn random_maybe_outlier_vec3_snorm<R: Rng + ?Sized>(rng: &mut R) -> Vec3 {
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
fn random_quat<R: Rng + ?Sized>(rng: &mut R) -> Quat {
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

fn random_outlier_quat<R: Rng + ?Sized>(rng: &mut R) -> Quat {
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
fn random_maybe_outlier_quat<R: Rng + ?Sized>(rng: &mut R) -> Quat {
    if rng.gen::<bool>() {
        random_quat(rng)
    } else {
        random_outlier_quat(rng)
    }
}

fn random_transform<R: Rng + ?Sized>(rng: &mut R) -> Transform {
    let translation = random_maybe_outlier_vec3_snorm(rng) * 0.5;
    let rotation: Quat = random_maybe_outlier_quat(rng);
    let scale = random_maybe_outlier_vec3_snorm(rng);

    Transform {
        translation,
        rotation,
        scale,
    }
}

enum RandomMeshError {
    InvalidNumJoints,
}

fn create_random_mesh<R: Rng + ?Sized>(
    rng: &mut R,
    num_tris: usize,
    num_joints: usize,
    max_influences: Option<usize>,
) -> Result<Mesh, RandomMeshError> {
    let max_influences = max_influences.unwrap_or(MAX_INFLUENCES).min(MAX_INFLUENCES);

    let num_joints = JointIndex::try_from(num_joints).or(Err(RandomMeshError::InvalidNumJoints))?;

    let position_dist = Uniform::new_inclusive(-0.5, 0.5);
    let joint_index_dist = Uniform::new(0, num_joints);
    let joint_weight_dist = Uniform::new(0.01, 1.0);
    let num_influences_dist = Uniform::new_inclusive(1, max_influences);

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
struct RandomMeshAnimation {
    noise: NoiseTimeline,
}

impl RandomMeshAnimation {
    fn new(seed: u64) -> Self {
        RandomMeshAnimation {
            noise: NoiseTimeline { seed },
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_random_mesh<R: Rng + ?Sized>(
    rng: &mut R,
    commands: &mut Commands,
    mesh_assets: &mut ResMut<Assets<Mesh>>,
    inverse_bindposes_assets: &mut ResMut<Assets<SkinnedMeshInverseBindposes>>,
    base: Entity,
    transform: Transform,
    material: MeshMaterial3d<StandardMaterial>,
    num_tris: usize,
    num_joints: usize,
    max_influences: Option<usize>,
) -> Result<(), RandomMeshError> {
    if num_joints == 0 {
        return Err(RandomMeshError::InvalidNumJoints);
    }

    let mesh_handle = mesh_assets.add(create_random_mesh(
        rng,
        num_tris,
        num_joints,
        max_influences,
    )?);

    // Create random inverse bindposes, but leave the root as identity so it's more visually pleasing.

    let random_transform_iter = repeat_with(|| random_transform(rng).compute_matrix());

    let inverse_bindposes: Vec<Mat4> = once(Mat4::IDENTITY)
        .chain(random_transform_iter)
        .take(num_joints)
        .collect();

    let inverse_bindposes_handle = inverse_bindposes_assets.add(inverse_bindposes);

    let mut joints: Vec<Entity> = Vec::with_capacity(num_joints);

    let root_joint = commands
        .spawn((Transform::IDENTITY, RandomMeshAnimation::new(rng.gen())))
        .set_parent(base)
        .id();

    joints.push(root_joint);

    for _ in 1..num_joints {
        let joint = commands
            .spawn((Transform::IDENTITY, RandomMeshAnimation::new(rng.gen())))
            .set_parent(root_joint)
            .id();

        joints.push(joint);
    }

    commands
        .spawn((
            transform,
            Mesh3d(mesh_handle),
            material,
            SkinnedMesh {
                inverse_bindposes: inverse_bindposes_handle,
                joints,
            },
        ))
        .set_parent(base);

    Ok(())
}

fn spawn_random_meshes(
    mut commands: Commands,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
    mut inverse_bindposes_assets: ResMut<Assets<SkinnedMeshInverseBindposes>>,
) {
    let mut rng = ChaCha8Rng::seed_from_u64(732935);

    let material = MeshMaterial3d(material_assets.add(StandardMaterial {
        base_color: Color::WHITE,
        cull_mode: None,
        ..default()
    }));

    struct MeshInstance {
        num_tris: usize,
        num_joints: usize,
        max_influences: Option<usize>,
        translation: Vec3,
    }

    let mesh_instances = [
        MeshInstance {
            num_tris: 100,
            num_joints: 1,
            max_influences: Some(1),
            translation: Vec3::new(-3.0, 1.5, 0.0),
        },
        MeshInstance {
            num_tris: 100,
            num_joints: 10,
            max_influences: Some(1),
            translation: Vec3::new(0.0, 1.5, 0.0),
        },
        MeshInstance {
            num_tris: 100,
            num_joints: 100,
            max_influences: Some(1),
            translation: Vec3::new(3.0, 1.5, 0.0),
        },
        MeshInstance {
            num_tris: 100,
            num_joints: 1,
            max_influences: None,
            translation: Vec3::new(-3.0, -1.5, 0.0),
        },
        MeshInstance {
            num_tris: 100,
            num_joints: 10,
            max_influences: None,
            translation: Vec3::new(0.0, -1.5, 0.0),
        },
        MeshInstance {
            num_tris: 100,
            num_joints: 100,
            max_influences: None,
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

        let _ = spawn_random_mesh(
            &mut rng,
            &mut commands,
            &mut mesh_assets,
            &mut inverse_bindposes_assets,
            base_entity,
            mesh_transform,
            material.clone(),
            mesh_instance.num_tris,
            mesh_instance.num_joints,
            mesh_instance.max_influences,
        );
    }
}

fn update_random_meshes(
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
