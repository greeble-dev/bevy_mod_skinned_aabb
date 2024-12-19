$example = $args[0]
Start-Job -ScriptBlock {tracy-capture -f -o $env:Temp/skinned_aabb.tracy}
cargo run --profile=bench --example=$example --features bevy/trace_tracy
tracy-profiler $env:Temp/skinned_aabb.tracy

