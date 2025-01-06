# System Info

```
SystemInfo { os: "Windows 10 Pro", kernel: "19045", cpu: "AMD Ryzen 9 7900 12-Core Processor", core_count: "12", memory: "63.2 GiB" }        
AdapterInfo { name: "NVIDIA GeForce GT 1030", vendor: 4318, device: 7425, device_type: DiscreteGpu, driver: "NVIDIA", driver_info: "551.86", backend: Vulkan }
```


# Many Foxes
- `cargo run --example=many_foxes --profile=bench --features bevy/trace_tracy --features trace`
- 1000 instances of Fox.glb
    - Mesh:
        - 576 verts.
        - 24 joints.
        - 22 skinned joints.
	- Scene:
    	- 24,000 joints.
    	- 22,000 skinned joints.
- Creation
	- `GltfLoader("Fox.glb")`
    	- 9.74ms
	- `create_skinned_aabb_asset("Fox.glb")`
    	- 15.81us
		- Time spent calculating AABBs for the mesh asset.
	- `system{name="bevy_mod_skinned_aabb::create_skinned_aabbs"}`
    	- 85.73us
		- Part of time spent making components/assets for all mesh instances.
	- `system_commands{name="bevy_mod_skinned_aabb::create_skinned_aabbs"}`
    	- 420us
		- Part of time spent making components/assets for all mesh instances.
- Updates
	- `schedule{name=PostUpdate}`
		- This span seems like a reasonable proxy for "main thread animated mesh time".
			- Includes animation, transform/visibility propagation, skinned AABBs.
			- Excludes render extraction and render thread.
		- Skinned AABBs disabled = 1.61ms
		- Skinned AABBs enabled = 1.67ms (x1.037)
	- `system{name="bevy_mod_skinned_aabb::update_skinned_aabbs"}` 
    	- Main function:
        	- 49.25us
        	- 2.24ns per skinned joint.
		- `par_iter`
    		- 175.55us (across all threads)
			- 8ns per skinned joint.
    		- Runs on 8 threads, but only ~45% occupancy.
            - Memory bandwidth = 2 + 24 + 8 + 64  = 98 bytes in 8ns = 12.25GB/s per thread.
                - Although 2 + 24 = 26 bytes is aabb_to_joint + aabb which is almost certain to be in cache.
                - So say 9.0GB/s.
                - DRAM bandwidth is ~70GB/s.
                - Would be bandwidth bound at ~7.8 threads.
                - Although not for this benchmark since the joints fit in L3 (22000 * 64 bytes = 1.4MB) and there's not much else competing for cache space.
        - Core loop over AABBs is 107 instructions, plus one call to QueryData::get_unchecked_manual (~50 instructions)
	- `system{name="bevy_mod_skinned_aabb::create_skinned_aabbs"}`
        - 600ns when no new meshes are found.
- Conclusions
    - Creation seems reasonable.
        - Calculating the bounds is a tiny percentage of GLTF load.
        - Creating the components and checking for new meshes every frame is not great.
            - But in long-term that disappears into the asset pipeline.
    - Update is ok-ish.
        - ~4% increase in overall time spent on animated meshes on main thread.
            - Not great, not terrible.
        - Will look better in cases with more animation blending - many_foxes is just sampling a single animation.
            - But worse if animation update and transform propagation get further optimisations.
            - Currently 29ns per joint when sampling a single animation - seems high?
        - Unknowns?
            - Assuming thread occupancy is increased, when do we become bandwidth/latency bound?
            - Only tested with 1000 foxes, should try more extreme numbers?
            - Are each mesh's joints contiguous in memory?
        - Guessing the main issue is the way things are grouped into systems.
            - The major systems are the animation update, transform propagate, and skinned aabb update.
            - Each system is mostly reading or writing joint transforms, and each mesh is independent.
            - So ideally each system would run per-mesh back to back on the same thread - the joints stay in L1/L2 and there's no waiting around.
            - But in reality each system waits for the entire previous system to finish, and there's no correlation between mesh and thread.
            - So there's lots of bubbles and joints are being read from L3.
            - Needs more testing to be sure.
        - Assuming systems are the issue, is it solvable?
            - Doesn't seem practical for the ECS to work out that each skinned mesh is independent.
            - Animation targets and skinned mesh joint tables could in theory access any entity.
                - And there's practical use cases where they will, even if many_foxes and other game-like situations won't.
            - Transform propagation is for all entities, not just joints.
            - So we'd need to:
                - Mark certain entity sub-trees as independent and then verify/enforce that invariant.
                - Optionally guarantee that systems don't read/write outside a certain sub-tree, and make transform propagation clever enough to separate these out.
                - And then express that various systems can run in a certain order on independent sub-trees.

