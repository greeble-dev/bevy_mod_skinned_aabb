# Bevy Skinned AABBs

***UNDER CONSTRUCTION - USE AT YOUR OWN RISK***

A [Bevy](https://github.com/bevyengine/bevy) plugin that automatically calculates AABBs for skinned meshes. The goal is to work around issues with disappearing skinned meshes (https://github.com/bevyengine/bevy/issues/4971).

https://github.com/user-attachments/assets/73d236da-43a8-4b63-a19e-f3625d374077

## Quick Start

To enable skinned AABBs in your Bevy app, first update your `Cargo.toml` dependencies:

```toml
[dependencies]
bevy_mod_skinned_aabb = { git = "https://github.com/greeble-dev/bevy_mod_skinned_aabb.git" }
```

Then add the plugin to your app:

```rust
use bevy_mod_skinned_aabb::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SkinnedAabbPlugin)
        .run();
}
```

The plugin will automatically detect and update any skinned meshes that are added to the world.

## Debug Rendering

To see mesh and joint AABBs, use `SkinnedAabbDebugPlugin`:

```rust
use bevy_mod_skinned_aabb::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((
            SkinnedAabbPlugin,
            SkinnedAabbDebugPlugin::disable_by_default(), // Add the debug rendering plugin.
        ))
        .add_systems(
            Update,
            (
                toggle_draw_joint_aabbs.run_if(input_just_pressed(KeyCode::KeyJ)), // Press J to toggle joint AABBs.
                toggle_draw_mesh_aabbs.run_if(input_just_pressed(KeyCode::KeyM)), // Press M to toggle mesh AABBs.
            ),
        )
        .run();	
}
```

## Try The Examples

```sh
git clone https://github.com/greeble-dev/bevy_mod_skinned_aabb
cd bevy_mod_skinned_aabb

# Functionality test of GLTF and procedural meshes.
cargo run --example=showcase

# Stress test of 1000 skinned meshes.
cargo run --example=many_foxes
```

## Limitations

- Enabling skinned AABBs increases the main thread CPU cost of skinned meshes by roughly 4%. 
    - Raw notes in [notes/Performance.md](notes/Performance.md).
- Skinned AABBs do **not** account for blend shapes and vertex shader deformations.
    - Meshes that use these features may have incorrect AABBs.
    - Meshes that only use skinning are safe.
- Skinned AABBs are conservative but not accurate.
    - They're conservative in that the AABB is guaranteed to contain the mesh's vertices.
    - But they're not accurate, in that the AABB may be larger than is necessary.
- The plugin requires that the main thread can access mesh vertices.
    - This appears to work fine by default, but might fail if asset settings are changed.
    - See https://github.com/bevyengine/bevy/blob/main/crates/bevy_asset/src/render_asset.rs.
- After spawning meshes, the AABBs might be wrong for one frame.
- If a mesh asset changes after being created/loaded then the skinned AABBs will not reflect the changes.
    - Deleting the SkinnedAabb component from the mesh's entity may fix this (untested).
    - TODO: How to address this properly? Note that Bevy has the same problem with regular AABBs (https://github.com/bevyengine/bevy/issues/4294).

## Bevy Compatibility

The main branch is compatible with Bevy 0.15.

TODO: Proper version tags, compatibility matrix.
