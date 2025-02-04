# Bevy Skinned AABBs

***UNDER CONSTRUCTION - USE AT YOUR OWN RISK***

A [Bevy](https://github.com/bevyengine/bevy) plugin that automatically calculates AABBs for skinned meshes.

https://github.com/user-attachments/assets/73d236da-43a8-4b63-a19e-f3625d374077

The goal of the plugin is to fix meshes disappearing due to incorrect AABBs (https://github.com/bevyengine/bevy/issues/4971).

---

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

# Show a variety of glTF and custom meshes.
cargo run --example showcase

# Stress test 1000 skinned meshes.
cargo run --example many_foxes
```

## Limitations

- Skinned AABBs do not account for blend shapes and vertex shader shenanigans.
    - Meshes that use these features may have incorrect AABBs.
    - Meshes that only use skinning are safe.
- Skinned AABBs are conservative but not accurate.
    - They're conservative in that the AABB is guaranteed to contain the mesh's vertices.
    - But they're not accurate, in that the AABB may be larger than is necessary.
- Apps that use hundreds of different skinned mesh assets may have performance issues.
    - Each different asset adds some overhead to spawning mesh instances.
    - It's fine to spawn many instances of a small number of assets.
- After spawning meshes, the AABBs might be wrong for one frame.

## Bevy Compatibility

The main branch is compatible with Bevy 0.15.

TODO: Proper version tags, compatibility matrix.

## FAQ

### What's the performance impact?

For meshes that are playing a single animation, skinned AABBs increase the per-frame
CPU cost of each mesh by roughly 4%.

The CPU cost of spawning a mesh from a glTF increases by less than 1%.

### How can I see the AABBs?

To see the mesh and joint AABBs in your own app, add `SkinnedAabbDebugPlugin`:

```rust
use bevy_mod_skinned_aabb::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((
            SkinnedAabbPlugin,
            SkinnedAabbDebugPlugin::enable_by_default(), // Enable debug rendering.
        ))
        .run();	
}
```

The debug rendering will be enabled by default. You can also leave it disabled by default but enable it with keyboard shortcuts:

```rust
use bevy_mod_skinned_aabb::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((
            SkinnedAabbPlugin,
            SkinnedAabbDebugPlugin::disable_by_default(),
        ))
        .add_systems(
            Update,
            (
                // Press J to toggle joint AABBs.
                toggle_draw_joint_aabbs.run_if(input_just_pressed(KeyCode::KeyJ)),
                // Press M to toggle mesh AABBs.
                toggle_draw_mesh_aabbs.run_if(input_just_pressed(KeyCode::KeyM)),
            ),
        )
        .run();	
}
```
