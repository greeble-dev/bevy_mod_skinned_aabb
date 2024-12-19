use bevy_app::{App, Plugin, PostUpdate, Update};
use bevy_asset::{Asset, AssetApp, AssetId, Assets, Handle};
use bevy_ecs::prelude::*;
use bevy_math::{
    bounding::{Aabb3d, BoundingVolume},
    Affine3A, Vec3, Vec3A,
};
use bevy_mesh::Mesh;
use bevy_reflect::prelude::*;
use bevy_render::{
    mesh::{
        skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
        Mesh3d, VertexAttributeValues,
    },
    primitives::Aabb,
    view::VisibilitySystems,
};
use bevy_transform::{components::GlobalTransform, TransformSystem};

pub mod debug;

pub mod prelude {
    pub use crate::debug::prelude::*;
    pub use crate::SkinnedBoundsPlugin;
}

#[derive(Default)]
pub struct SkinnedBoundsPlugin;

impl Plugin for SkinnedBoundsPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<SkinnedBoundsAsset>()
            .add_systems(Update, create_skinned_bounds)
            .add_systems(
                PostUpdate,
                update_skinned_bounds
                    // TODO: Verify ordering.
                    .after(TransformSystem::TransformPropagate)
                    .before(VisibilitySystems::CheckVisibility),
            );
    }
}

// Match the Mesh limits on joint indices (ATTRIBUTE_JOINT_INDEX = VertexFormat::Uint16x4)
type JointIndex = u16;

// An Aabb3d without padding.
#[derive(Copy, Clone, Debug, Reflect)]
pub struct PackedAabb3d {
    pub min: Vec3,
    pub max: Vec3,
}

impl From<PackedAabb3d> for Aabb3d {
    fn from(value: PackedAabb3d) -> Self {
        Self {
            min: value.min.into(),
            max: value.max.into(),
        }
    }
}

impl From<Aabb3d> for PackedAabb3d {
    fn from(value: Aabb3d) -> Self {
        Self {
            min: value.min.into(),
            max: value.max.into(),
        }
    }
}

#[derive(Asset, Debug, TypePath)]
pub struct SkinnedBoundsAsset {
    // The source mesh and inverse bindpose assets. We keep these so that entities can
    // reuse existing SkinnedBoundsAssets.
    pub mesh: AssetId<Mesh>,
    pub inverse_bindposes: AssetId<SkinnedMeshInverseBindposes>,

    // Bounds for skinned joints.
    pub bounds: Box<[PackedAabb3d]>,

    // Mapping from bounds index to SkinnedMesh::joints index.
    pub bound_to_joint: Box<[JointIndex]>,
}

impl SkinnedBoundsAsset {
    pub fn aabb(&self, bound_index: usize) -> Aabb3d {
        self.bounds[bound_index].into()
    }

    pub fn world_from_joint(
        &self,
        bound_index: usize,
        skinned_mesh: &SkinnedMesh,
        joints: &Query<&GlobalTransform>,
    ) -> Option<Affine3A> {
        // TODO: Should return an error instead of silently failing?
        let joint_index = *self.bound_to_joint.get(bound_index)? as usize;
        let joint_entity = *skinned_mesh.joints.get(joint_index)?;

        Some(joints.get(joint_entity).ok()?.affine())
    }

    pub fn num_bounds(&self) -> usize {
        self.bounds.len()
    }
}

#[derive(Component, Debug, Default)]
pub struct SkinnedBounds {
    // Optional asset. This is optional because the skinned bounds can fail to create due to missing
    // assets, but we still need to add the component so we don't attempt to recreate it next frame.
    //
    // If the skinned bounds creation is moved into the asset pipeline then this doesn't need to be optional.
    pub asset: Option<Handle<SkinnedBoundsAsset>>,
}

// Return an aabb that contains the given point and optional aabb.
fn merge(aabb: Option<Aabb3d>, point: Vec3A) -> Aabb3d {
    match aabb {
        Some(aabb) => Aabb3d {
            min: point.min(aabb.min),
            max: point.max(aabb.max),
        },
        None => Aabb3d {
            min: point,
            max: point,
        },
    }
}

struct Influence {
    position: Vec3,
    joint_index: usize,
}

struct InfluenceIterator<'a> {
    vertex_index: usize,
    influence_index: usize,
    positions: &'a [[f32; 3]],
    joint_indices: &'a [[u16; 4]],
    joint_weights: &'a [[f32; 4]],
}

/// Iterates over all vertex influences with non-zero weight.
impl Default for InfluenceIterator<'_> {
    fn default() -> Self {
        InfluenceIterator {
            vertex_index: 0,
            influence_index: 0,
            positions: &[],
            joint_indices: &[],
            joint_weights: &[],
        }
    }
}

impl<'a> InfluenceIterator<'a> {
    fn new(mesh: &'a Mesh) -> Self {
        if let (
            Some(VertexAttributeValues::Float32x3(positions)),
            Some(VertexAttributeValues::Uint16x4(joint_indices)),
            Some(VertexAttributeValues::Float32x4(joint_weights)),
        ) = (
            mesh.attribute(Mesh::ATTRIBUTE_POSITION),
            mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX),
            mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT),
        ) {
            if (joint_indices.len() != positions.len()) | (joint_weights.len() != positions.len()) {
                // TODO: Should be an error?
                return InfluenceIterator::default();
            }

            return InfluenceIterator {
                vertex_index: 0,
                influence_index: 0,
                positions,
                joint_indices,
                joint_weights,
            };
        }

        InfluenceIterator::default()
    }
}

impl Iterator for InfluenceIterator<'_> {
    type Item = Influence;

    fn next(&mut self) -> Option<Influence> {
        loop {
            // TODO: Bit janky hard-coding the 3 here. Will need refactoring anyway once Bevy
            // supports > 4 influences.
            if self.influence_index > 3 {
                self.influence_index = 0;
                self.vertex_index += 1;
            }

            if self.vertex_index >= self.positions.len() {
                break None;
            }

            let position = Vec3::from_array(self.positions[self.vertex_index]);
            let joint_index = self.joint_indices[self.vertex_index][self.influence_index];
            let joint_weight = self.joint_weights[self.vertex_index][self.influence_index];

            self.influence_index += 1;

            if joint_weight > 0.0 {
                break Some(Influence {
                    position,
                    joint_index: joint_index as usize,
                });
            }
        }
    }
}

fn create_skinned_bounds_asset(
    mesh: &Mesh,
    mesh_handle: AssetId<Mesh>,
    inverse_bindposes: &SkinnedMeshInverseBindposes,
    inverse_bindposes_handle: AssetId<SkinnedMeshInverseBindposes>,
) -> SkinnedBoundsAsset {
    let num_joints = inverse_bindposes.len();

    // TODO: Error if num_joints exceeds JointIndex limits?

    // Calculate the jointspace bounds for each joint.

    let mut optional_bounds: Vec<Option<Aabb3d>> = vec![None; num_joints];

    for Influence {
        position,
        joint_index,
    } in InfluenceIterator::new(mesh)
    {
        assert!(joint_index < optional_bounds.len());

        let jointspace_position = inverse_bindposes[joint_index].transform_point3(position);

        optional_bounds[joint_index] = Some(merge(
            optional_bounds[joint_index],
            Vec3A::from(jointspace_position),
        ));
    }

    // Filter out any joints without bounds.

    let num_bounds = optional_bounds.iter().filter(|o| o.is_some()).count();

    let mut bounds = Vec::<PackedAabb3d>::with_capacity(num_bounds);
    let mut bound_to_joint = Vec::<JointIndex>::with_capacity(num_bounds);

    for (joint_index, _) in optional_bounds.iter().enumerate() {
        if let Some(bound) = optional_bounds[joint_index] {
            bounds.push(bound.into());
            bound_to_joint.push(joint_index as JointIndex);
        }
    }

    assert!(bounds.len() == num_bounds);
    assert!(bound_to_joint.len() == num_bounds);

    SkinnedBoundsAsset {
        mesh: mesh_handle,
        inverse_bindposes: inverse_bindposes_handle,
        bounds: bounds.into(),
        bound_to_joint: bound_to_joint.into(),
    }
}

fn create_skinned_bounds_component(
    skinned_bounds_assets: &mut ResMut<Assets<SkinnedBoundsAsset>>,
    mesh_assets: &Res<Assets<Mesh>>,
    mesh_handle: &Handle<Mesh>,
    inverse_bindposes_assets: &Res<Assets<SkinnedMeshInverseBindposes>>,
    inverse_bindposes_handle: &Handle<SkinnedMeshInverseBindposes>,
) -> SkinnedBounds {
    // First check for an existing asset.

    for (existing_asset_id, existing_asset) in skinned_bounds_assets.iter() {
        if (existing_asset.mesh == mesh_handle.id())
            & (existing_asset.inverse_bindposes == inverse_bindposes_handle.id())
        {
            return SkinnedBounds {
                asset: Some(Handle::Weak(existing_asset_id)),
            };
        }
    }

    // No existing asset so create one.

    if let (Some(mesh), Some(inverse_bindposes)) = (
        mesh_assets.get(mesh_handle),
        inverse_bindposes_assets.get(inverse_bindposes_handle),
    ) {
        let asset = create_skinned_bounds_asset(
            mesh,
            mesh_handle.id(),
            inverse_bindposes,
            inverse_bindposes_handle.id(),
        );

        let asset_handle = skinned_bounds_assets.add(asset);

        return SkinnedBounds {
            asset: Some(asset_handle),
        };
    }

    SkinnedBounds { asset: None }
}

fn create_skinned_bounds(
    mut commands: Commands,
    mesh_assets: Res<Assets<Mesh>>,
    inverse_bindposes_assets: Res<Assets<SkinnedMeshInverseBindposes>>,
    mut skinned_bounds_assets: ResMut<Assets<SkinnedBoundsAsset>>,
    query: Query<(Entity, &Mesh3d, &SkinnedMesh), Without<SkinnedBounds>>,
) {
    for (entity, mesh, skinned_mesh) in &query {
        let skinned_bounds = create_skinned_bounds_component(
            &mut skinned_bounds_assets,
            &mesh_assets,
            &mesh.0,
            &inverse_bindposes_assets,
            &skinned_mesh.inverse_bindposes,
        );

        commands.entity(entity).try_insert(skinned_bounds);
    }
}

/// Scalar version of aabb_transformed_by, kept here for reference.
///
/// Algorithm from "Transforming Axis-Aligned Bounding Boxes", James Arvo, Graphics Gems (1990).
///
/// TODO: Benchmark against the simd version? Worth a check in case the compiler is cleverer.
#[cfg(any())]
fn aabb_transformed_by_scalar(input: Aabb3d, transform: Affine3A) -> Aabb3d {
    let rs = transform.matrix3.to_cols_array_2d();
    let t = transform.translation;

    let mut min = t;
    let mut max = t;

    for i in 0..3 {
        for j in 0..3 {
            let e = rs[j][i] * input.min[j];
            let f = rs[j][i] * input.max[j];

            min[i] += e.min(f);
            max[i] += e.max(f);
        }
    }

    return Aabb3d { min, max };
}

fn aabb_transformed_by(input: Aabb3d, transform: Affine3A) -> Aabb3d {
    let rs = transform.matrix3;
    let t = transform.translation;

    let input_min_x = Vec3A::splat(input.min.x);
    let input_min_y = Vec3A::splat(input.min.y);
    let input_min_z = Vec3A::splat(input.min.z);

    let input_max_x = Vec3A::splat(input.max.x);
    let input_max_y = Vec3A::splat(input.max.y);
    let input_max_z = Vec3A::splat(input.max.z);

    let e_x = rs.x_axis * input_min_x;
    let e_y = rs.y_axis * input_min_y;
    let e_z = rs.z_axis * input_min_z;

    let f_x = rs.x_axis * input_max_x;
    let f_y = rs.y_axis * input_max_y;
    let f_z = rs.z_axis * input_max_z;

    let min_x = e_x.min(f_x);
    let min_y = e_y.min(f_y);
    let min_z = e_z.min(f_z);

    let max_x = e_x.max(f_x);
    let max_y = e_y.max(f_y);
    let max_z = e_z.max(f_z);

    let min = t + min_x + min_y + min_z;
    let max = t + max_x + max_y + max_z;

    // TODO: Should we mask off the w before storing? Check what Vec3A is expecting - we might
    // have to switch to Vec4.

    Aabb3d { min, max }
}

fn get_skinned_aabb(
    bounds: &SkinnedBounds,
    joints: &Query<&GlobalTransform>,
    assets: &Res<Assets<SkinnedBoundsAsset>>,
    skinned_mesh: &SkinnedMesh,
    world_from_entity: &Affine3A,
) -> Option<Aabb> {
    let asset = assets.get(bounds.asset.as_ref()?)?;

    let num_bounds = asset.num_bounds();

    if num_bounds == 0 {
        return None;
    }

    let entity_from_world = world_from_entity.inverse();

    let mut entity_aabb = Aabb3d {
        min: Vec3A::MAX,
        max: Vec3A::MIN,
    };

    for bound_index in 0..num_bounds {
        if let Some(world_from_joint) = asset.world_from_joint(bound_index, skinned_mesh, joints) {
            let entity_from_joint = entity_from_world * world_from_joint;
            let joint_aabb = aabb_transformed_by(asset.aabb(bound_index), entity_from_joint);

            entity_aabb = entity_aabb.merge(&joint_aabb);
        }
    }

    // If min > max then no joints were found.

    if entity_aabb.min.x > entity_aabb.max.x {
        None
    } else {
        Some(Aabb::from_min_max(
            Vec3::from(entity_aabb.min),
            Vec3::from(entity_aabb.max),
        ))
    }
}

fn update_skinned_bounds(
    mut query: Query<(&mut Aabb, &SkinnedBounds, &SkinnedMesh, &GlobalTransform)>,
    joints: Query<&GlobalTransform>,
    assets: Res<Assets<SkinnedBoundsAsset>>,
) {
    query
        .par_iter_mut()
        .for_each(|(mut entity_aabb, bounds, skinned_mesh, world_from_mesh)| {
            if let Some(skinned_aabb) = get_skinned_aabb(
                bounds,
                &joints,
                &assets,
                skinned_mesh,
                &world_from_mesh.affine(),
            ) {
                *entity_aabb = skinned_aabb
            }
        })
}
