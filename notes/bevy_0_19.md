# Bevy 0.19 and `bevy_mod_skinned_aabb`

Bevy 0.19 [adds support](https://github.com/bevyengine/bevy/pull/21837) for
dynamic skinned mesh bounds, so `bevy_mod_skinned_aabb` is no longer required!

## Upgrading

When upgrading to Bevy 0.19, first remove the `bevy_mod_skinned_aabb` plugin:

```sh
cargo remove bevy_mod_skinned_aabb
```
```diff
 App::new()
-    .add_plugins(SkinnedAabbPlugin)
```

If your meshes come from the glTF loader then that's all you need to do - the 
glTF loader will automatically enable dynamic skinned mesh bounds.

If you create your skinned meshes without using the glTF loader, you'll need to
call `Mesh::generate_skinned_mesh_bounds` or
`Mesh::with_generated_skinned_bounds` and add a `DynamicSkinnedMeshBounds`
component to your mesh entity.

```diff
 let mut mesh = ...;
+mesh.generate_skinned_mesh_bounds()?;
 
 entity.insert((
     Mesh3d(meshes.add(mesh)),
+    DynamicSkinnedMeshBounds,
 ));
```

If you use the debug visualizations of `SkinnedAabbDebugPlugin`, they can be
replaced by Bevy's gizmos:

```rust
fn toggle_skinned_mesh_bounds(mut config: ResMut<GizmoConfigStore>) {
	// Toggle drawing of the per-mesh `Aabb` component that's used for culling.
    config.config_mut::<AabbGizmoConfigGroup>().1.draw_all ^= true;
	// Toggle drawing of the per-joint AABBs used to update the `Aabb` component.
    config.config_mut::<SkinnedMeshBoundsGizmoConfigGroup>().1.draw_all ^= true;
}
```

## Differences

Bevy's implementation is slightly different to `bevy_mod_skinned_aabb`. This
makes it faster and more reliable, but the calculated AABBs might change.

If you only used `bevy_mod_skinned_aabb` to fix frustum culling then you're very
unlikely to notice a difference. If you used `bevy_mod_skinned_aabb` for
collision or picking then you might find that the accuracy has increased or
decreased.
