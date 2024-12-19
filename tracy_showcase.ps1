Start-Job -ScriptBlock {tracy-capture -f -o $env:Temp/showcase.tracy}
cargo run --profile=bench --example=showcase --features bevy/trace_tracy
tracy-profiler $env:Temp/showcase.tracy
