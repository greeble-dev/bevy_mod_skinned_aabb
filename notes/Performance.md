# System Info

```
SystemInfo { os: "Windows 10 Pro", kernel: "19045", cpu: "AMD Ryzen 9 7900 12-Core Processor", core_count: "12", memory: "63.2 GiB" }        
AdapterInfo { name: "NVIDIA GeForce GT 1030", vendor: 4318, device: 7425, device_type: DiscreteGpu, driver: "NVIDIA", driver_info: "551.86", backend: Vulkan }
```


# Many Foxes
- `cargo run --example=many_foxes --profile=bench --features bevy/trace_tracy --features trace`
- 1000 instances of Fox.glb
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
			- Excludes render extraction and processing.
		- Skinned AABBs disabled = 1.61ms
		- Skinned AABBs enabled = 1.67ms (x1.037)
	- `system{name="bevy_mod_skinned_aabb::update_skinned_aabbs"}` 
    	- 49.25us
		- 2.24ns per skinned joint
		- `par_iter` time
    		- 175.55us (across all threads)
			- 8ns per skinned joint.
    		- Runs on 8 cores, but only effectively utilises ~3.5.
            - Memory bandwidth = 2 + 8 + 64 + 24 = 98 bytes in 8ns = 12.25GB/s per core.
- Conclusions
    - Creation seems reasonable.
        - Actually calculating the bounds is a tiny percentage of GLTF load.
        - Creating the components is not great, but in long-term that gets merged into asset pipeline and existing skinned mesh component?
    - Update is ok-ish.
        - ~4% increase in overall time spent on animated meshes on main thread.
        - Not great, not terrible.
        - Will look better on things doing more animation blending - many_foxes is just sampling a single animation.
        - Might look worse if animation and transform propagation get further optimisations.
        - Main issue is that it's an independent system.
            - Not making good use of cores and L1/2.
            - Doesn't exploit temporal locality of joint transforms across the various systems (animation blend -> transform propagate -> skinned aabb -> write gpu buffers)
