# Runs a Tracy capture on an example and then starts the Tracy UI.
#
# Usage: tracy.ps1 <example> <optional_postfix>
#
# The file is saved to target/<example>_<optional_postfix>.tracy.
# 
# TODO: Pass remaining arguments to example?

$example = $args[0]
$postfix = ""

if ( $args.count -gt 1 )
{
	$postfix = "_" + $args[1]
}

$file = "${pwd}/target/${example}${postfix}.tracy"

Start-Job -ScriptBlock { tracy-capture -f -o $using:file }
cargo run --example=$example --profile=bench --features bevy/trace_tracy --features trace
Get-Job | Wait-Job
echo "Profile saved to `"${file}`""
tracy-profiler $file

