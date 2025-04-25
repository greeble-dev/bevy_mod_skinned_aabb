# bevy_mod_skinned_aabb

A [Bevy](https://github.com/bevyengine/bevy) plugin that automatically calculates AABBs for skinned meshes.

https://github.com/user-attachments/assets/73d236da-43a8-4b63-a19e-f3625d374077

The goal of the plugin is to [fix meshes disappearing due to incorrect AABBs](https://github.com/bevyengine/bevy/issues/4971).

## Quick Start

To enable skinned AABBs in a Bevy 0.16 app:

```sh
cargo add bevy_mod_skinned_aabb
```

```rust
use bevy_mod_skinned_aabb::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Enable skinned AABBs.
        .add_plugins(SkinnedAabbPlugin)
        .run();
}
```

The plugin will automatically detect and update any skinned meshes that are added to the world.

## Bevy Compatibility

| bevy          | bevy_mod_skinned_aabb |
|---------------|-----------------------|
| `0.16.0`      | `0.2`                 |
| `0.16.0-rc.4` | `0.2.0-rc.4`          |
| `0.16.0-rc.1` | `0.2.0-rc.1`          |
| `0.15`        | `0.1`                 |
| `<=0.14`      | Not supported         |

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
- Skinned AABBs are conservative but not optimal.
    - They're conservative in that the AABB is guaranteed to contain the mesh's vertices.
    - But they're not optimal, in that the AABB may be larger than necessary.
- Apps that use hundreds of different skinned mesh assets may have performance issues.
    - Each different asset adds some overhead to spawning mesh instances.
    - It's fine to spawn many instances of a small number of assets.
- The AABBs might be wrong for one frame immediately after spawning.

## FAQ

### What's the performance impact?

The per-frame CPU cost of a skinned mesh increases by roughly 4%. The
cost of loading a skinned mesh from a glTF increases by less than 1%.

### How can I see the AABBs?

To see the mesh and joint AABBs in your own app, add `SkinnedAabbDebugPlugin`:

```rust
use bevy_mod_skinned_aabb::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((
            SkinnedAabbPlugin,
            // Enable debug rendering.
            SkinnedAabbDebugPlugin::enable_by_default(),
        ))
        .run();	
}
```

The debug rendering will be enabled by default. You can also leave it disabled
by default but enable it with keyboard shortcuts:

```rust
use bevy_mod_skinned_aabb::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((
            SkinnedAabbPlugin,
            // Add the debug rendering but leave it disabled by default.
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
