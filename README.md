# Bevy Skinned AABBs

***UNDER CONSTRUCTION - USE AT YOUR OWN RISK***

A [Bevy](https://github.com/bevyengine/bevy) plugin that automatically calculates AABBs for skinned meshes. This can solve issues with disappearing meshes (https://github.com/bevyengine/bevy/issues/4971).

https://github.com/user-attachments/assets/73d236da-43a8-4b63-a19e-f3625d374077


The plugin calculates an AABB for each joint (white boxes), then uses them to calculate an AABB for the whole skinned mesh (colored boxes).

- [Quick Start](#quick-start)
- [Examples](#examples)
- [Limitations](#limitations)
- [Bevy Compatibility](#bevy-compatibility)
- [FAQ](#faq)


## Quick Start

To enable skinned AABBs in your Bevy app, update your `Cargo.toml` dependencies:

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

## Examples

```sh
git clone https://github.com/greeble-dev/bevy_mod_skinned_aabb
cd bevy_mod_skinned_aabb

# Show a variety of glTF and procedural meshes.
cargo run --example=showcase

# Stress test 1000 skinned meshes.
cargo run --example=many_foxes
```

## Limitations

The plugin is intended for Bevy apps that spawn a moderate number of custom or glTF skinned meshes.

Other apps might run into some limitations:

- Skinned AABBs do not account for blend shapes and vertex shader shenanigans.
    - Meshes that use these features may have incorrect AABBs.
    - Meshes that only use skinning are safe.
- Enabling skinned AABBs increases the main thread CPU cost of skinned meshes by roughly 4%. 
    - Raw notes in [notes/Performance.md](notes/Performance.md).
- Apps that spawn thousands of different assets may have performance issues.
    - Spawning many instances of a moderate number of assets is fine.
- Skinned AABBs are conservative but not accurate.
    - They're conservative in that the AABB is guaranteed to contain the mesh's vertices.
    - But they're not accurate, in that the AABB may be larger than is necessary.
- The plugin requires that the main world has access to mesh data.
    - This should only be a problem for users that are creating meshes directly in the render world.
- After spawning meshes, the AABBs might be wrong for one frame.
- If a mesh asset changes after being spawned then the skinned AABBs will not reflect the changes.
    - Deleting the SkinnedAabb component from the mesh's entity may fix this (untested).
    - TODO: How to address this properly? Note that Bevy has the same problem with regular AABBs (https://github.com/bevyengine/bevy/issues/4294).

## Bevy Compatibility

The main branch is compatible with Bevy 0.15.

TODO: Proper version tags, compatibility matrix.

## FAQ

### How can I see the AABBs like in the examples?

To see the mesh and joint AABBs in your own app, add `SkinnedAabbDebugPlugin`:

```rust
use bevy_mod_skinned_aabb::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((
            SkinnedAabbPlugin,
            SkinnedAabbDebugPlugin::enable_by_default(), // Add the debug rendering plugin, enabling the rendering by default.
        ))
        .run();	
}
```

Or add the plugin but enable the rendering with keyboard shortcuts:

```rust
use bevy_mod_skinned_aabb::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((
            SkinnedAabbPlugin,
            SkinnedAabbDebugPlugin::disable_by_default(), // Add the debug rendering plugin, disabling the rendering by default.
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

