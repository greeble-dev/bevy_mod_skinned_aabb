use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::Assets;
use bevy_camera::primitives::Aabb;
use bevy_color::{Color, Oklcha};
use bevy_ecs::{
    change_detection::{Res, ResMut},
    entity::Entity,
    query::With,
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::Query,
};
use bevy_gizmos::{AppGizmoBuilder, config::GizmoConfigGroup, gizmos::Gizmos};
use bevy_math::{Affine3A, Vec3A, bounding::Aabb3d};
use bevy_mesh::skinning::SkinnedMesh;
use bevy_reflect::Reflect;
use bevy_transform::{components::GlobalTransform, plugins::TransformSystems};

use crate::{SkinnedAabb, SkinnedAabbAsset};

pub mod prelude {
    pub use crate::debug::{
        SkinnedAabbDebugPlugin, toggle_draw_joint_aabbs, toggle_draw_mesh_aabbs,
    };
}

#[derive(Default)]
pub struct SkinnedAabbDebugPlugin {
    pub enable_by_default: bool,
}

impl SkinnedAabbDebugPlugin {
    pub fn new(enable_by_default: bool) -> Self {
        SkinnedAabbDebugPlugin { enable_by_default }
    }

    pub fn enable_by_default() -> Self {
        SkinnedAabbDebugPlugin {
            enable_by_default: true,
        }
    }

    pub fn disable_by_default() -> Self {
        SkinnedAabbDebugPlugin {
            enable_by_default: false,
        }
    }
}

#[derive(Default, Resource)]
pub struct SkinnedAabbDebugConfig {
    // If true, draw the aabbs of all skinned mesh joints.
    pub draw_joint_aabbs: bool,

    // If true, draw the aabbs of all entities that have a skinned aabb.
    pub draw_mesh_aabbs: bool,
}

impl Plugin for SkinnedAabbDebugPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SkinnedAabbDebugConfig {
            draw_joint_aabbs: self.enable_by_default,
            draw_mesh_aabbs: self.enable_by_default,
        })
        .init_gizmo_group::<SkinnedAabbGizmos>()
        .add_systems(
            PostUpdate,
            (
                draw_joint_aabbs
                    .after(TransformSystems::Propagate)
                    .run_if(|config: Res<SkinnedAabbDebugConfig>| config.draw_joint_aabbs),
                draw_mesh_aabbs
                    .after(TransformSystems::Propagate)
                    .run_if(|config: Res<SkinnedAabbDebugConfig>| config.draw_mesh_aabbs),
            ),
        );
    }
}

#[derive(Default, Reflect, GizmoConfigGroup)]
struct SkinnedAabbGizmos {}

pub fn toggle_draw_joint_aabbs(mut debug: ResMut<SkinnedAabbDebugConfig>) {
    debug.draw_joint_aabbs ^= true;
}

pub fn toggle_draw_mesh_aabbs(mut debug: ResMut<SkinnedAabbDebugConfig>) {
    debug.draw_mesh_aabbs ^= true;
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

fn draw_joint_aabbs(
    query: Query<(&SkinnedAabb, &SkinnedMesh)>,
    joints: Query<&GlobalTransform>,
    mut gizmos: Gizmos<SkinnedAabbGizmos>,
    assets: Res<Assets<SkinnedAabbAsset>>,
) {
    // TODO: Nesting a bit too deep? Maybe split into an inner function.

    query.iter().for_each(|(skinned_aabb, skinned_mesh)| {
        if let Some(asset) = assets.get(&skinned_aabb.asset) {
            for aabb_index in 0..asset.num_aabbs() {
                if let Some(world_from_joint) =
                    asset.world_from_joint(aabb_index, skinned_mesh, &joints)
                {
                    let joint_from_aabb =
                        gizmo_transform_from_aabb3d(asset.aabb(aabb_index).into());
                    let world_from_aabb = world_from_joint * joint_from_aabb;

                    gizmos.cube(world_from_aabb, Color::WHITE);
                }
            }
        }
    })
}

fn draw_mesh_aabbs(
    query: Query<(Entity, &Aabb, &GlobalTransform), With<SkinnedAabb>>,
    mut gizmos: Gizmos<SkinnedAabbGizmos>,
) {
    query.iter().for_each(|(entity, aabb, world_from_entity)| {
        let entity_from_aabb = gizmo_transform_from_aabb(*aabb);
        let world_from_aabb = world_from_entity.affine() * entity_from_aabb;
        let color = Oklcha::sequential_dispersed(entity.index_u32());

        gizmos.cube(world_from_aabb, color);
    })
}
