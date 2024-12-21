# Bevy Skinned AABBs

***UNDER CONSTRUCTION - USE AT YOUR OWN RISK***

A [Bevy](https://github.com/bevyengine/bevy) plugin that automatically calculates AABBs for skinned meshes. Intended to help workaround issues with disappearing skinned meshes (https://github.com/bevyengine/bevy/issues/4971).

https://github.com/user-attachments/assets/73d236da-43a8-4b63-a19e-f3625d374077

## Quick Start

```toml
# Cargo.toml

[dependencies]
bevy_mod_skinned_aabb = { git = "https://github.com/greeble-dev/bevy_mod_skinned_aabb.git" }
```

```rust
// main.rs

use bevy_mod_skinned_aabb::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SkinnedAabbPlugin)
        .run();
}
```

The plugin will automatically detect and update any skinned meshes that are added to the world, including GLTF imported meshes.

## Optionally Add Debug Rendering

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
                toggle_draw_joint_aabbs.run_if(input_just_pressed(KeyCode::KeyJ)),
                toggle_draw_mesh_aabbs.run_if(input_just_pressed(KeyCode::KeyM)),
            ),
        )
        .run();	
}
```

Toggle joint AABBs by pressing "J", and mesh AABBs by pressing "M".

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
- The skinned AABBs do **not** account for blend shapes and vertex shader deformations.
	- These meshes may still have visibility issues.
	- Meshes that only use skinning are safe.
- The plugin requires that the main thread can access mesh vertices.
	- This appears to work fine by default, but might fail if asset settings are changed.
	- See https://github.com/bevyengine/bevy/blob/main/crates/bevy_asset/src/render_asset.rs.
- After spawning meshes, there might be a one frame gap before the correct AABBs are calculated.
- If a mesh asset changes after being created/loaded then the skinned AABBs will not reflect the changes.
    - TODO: How to address this? Note that Bevy has the same problem with regular AABBs (https://github.com/bevyengine/bevy/issues/4294).

## Bevy Compatibility

The main branch is compatible with Bevy 0.15.

TODO: Proper version tags, compatibility matrix.