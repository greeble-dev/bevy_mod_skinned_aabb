use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::Assets;
use bevy_color::{Color, Oklcha};
use bevy_ecs::prelude::*;
use bevy_gizmos::{config::GizmoConfigGroup, gizmos::Gizmos, AppGizmoBuilder};
use bevy_math::{bounding::Aabb3d, Affine3A, Vec3A};
use bevy_reflect::Reflect;
use bevy_render::{mesh::skinning::SkinnedMesh, primitives::Aabb};
use bevy_transform::{components::GlobalTransform, TransformSystem};

use crate::{SkinnedBounds, SkinnedBoundsAsset};

pub mod prelude {
    pub use crate::debug::{
        toggle_draw_entity_bounds, toggle_draw_joint_bounds, SkinnedBoundsDebugPlugin,
    };
}

#[derive(Default)]
pub struct SkinnedBoundsDebugPlugin {
    pub enable_by_default: bool,
}

impl SkinnedBoundsDebugPlugin {
    pub fn new(enable_by_default: bool) -> Self {
        SkinnedBoundsDebugPlugin { enable_by_default }
    }

    pub fn enable_by_default() -> Self {
        SkinnedBoundsDebugPlugin {
            enable_by_default: true,
        }
    }

    pub fn disable_by_default() -> Self {
        SkinnedBoundsDebugPlugin {
            enable_by_default: false,
        }
    }
}

#[derive(Default, Resource)]
pub struct SkinnedBoundsDebugConfig {
    // If true, draw the bounds of all skinned mesh joints.
    pub draw_joints: bool,

    // If true, draw the bounds of all entities that have skinned bounds.
    pub draw_entities: bool,
}

impl Plugin for SkinnedBoundsDebugPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SkinnedBoundsDebugConfig {
            draw_joints: self.enable_by_default,
            draw_entities: self.enable_by_default,
        })
        .init_gizmo_group::<SkinnedBoundsGizmos>()
        .add_systems(
            PostUpdate,
            (
                draw_joint_bounds
                    .after(TransformSystem::TransformPropagate)
                    .run_if(|config: Res<SkinnedBoundsDebugConfig>| config.draw_joints),
                draw_entity_bounds
                    .after(TransformSystem::TransformPropagate)
                    .run_if(|config: Res<SkinnedBoundsDebugConfig>| config.draw_entities),
            ),
        );
    }
}

#[derive(Default, Reflect, GizmoConfigGroup)]
struct SkinnedBoundsGizmos {}

pub fn toggle_draw_joint_bounds(mut debug: ResMut<SkinnedBoundsDebugConfig>) {
    debug.draw_joints ^= true;
}

pub fn toggle_draw_entity_bounds(mut debug: ResMut<SkinnedBoundsDebugConfig>) {
    debug.draw_entities ^= true;
}

fn gizmo_transform_from_aabb(aabb: Aabb) -> Affine3A {
    let s = aabb.half_extents * 2.0;

    Affine3A::from_cols(
        Vec3A::new(s.x, 0.0, 0.0),
        Vec3A::new(0.0, s.y, 0.0),
        Vec3A::new(0.0, 0.0, s.z),
        aabb.center,
    )
}

fn gizmo_transform_from_aabb3d(aabb: Aabb3d) -> Affine3A {
    gizmo_transform_from_aabb(Aabb::from_min_max(aabb.min.into(), aabb.max.into()))
}

fn draw_joint_bounds(
    query: Query<(&SkinnedBounds, &SkinnedMesh)>,
    joints: Query<&GlobalTransform>,
    mut gizmos: Gizmos<SkinnedBoundsGizmos>,
    assets: Res<Assets<SkinnedBoundsAsset>>,
) {
    // TODO: Nesting a bit too deep? Maybe split into an inner function.

    query.iter().for_each(|(bounds, skinned_mesh)| {
        if let Some(asset_handle) = &bounds.asset {
            if let Some(asset) = assets.get(asset_handle) {
                for bound_index in 0..asset.num_bounds() {
                    if let Some(world_from_joint) =
                        asset.world_from_joint(bound_index, skinned_mesh, &joints)
                    {
                        let joint_from_aabb = gizmo_transform_from_aabb3d(asset.aabb(bound_index));
                        let world_from_aabb = world_from_joint * joint_from_aabb;

                        gizmos.cuboid(world_from_aabb, Color::WHITE);
                    }
                }
            }
        }
    })
}

fn draw_entity_bounds(
    query: Query<(Entity, &SkinnedBounds, &Aabb, &GlobalTransform)>,
    mut gizmos: Gizmos<SkinnedBoundsGizmos>,
) {
    query
        .iter()
        .for_each(|(entity, _, aabb, world_from_entity)| {
            let entity_from_aabb = gizmo_transform_from_aabb(*aabb);
            let world_from_aabb = world_from_entity.affine() * entity_from_aabb;
            let color = Oklcha::sequential_dispersed(entity.index());

            gizmos.cuboid(world_from_aabb, color);
        })
}
