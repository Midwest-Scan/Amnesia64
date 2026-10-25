# Plant one realistic Rust bug at a time and require the public C++ API
# differential test to detect it. Compile/runtime failures never count.
# Source bytes are restored in finally; the final clean four-way build also
# restores both libraries. Do not edit/build the Rust source concurrently.
# Usage: ./mutate.ps1 <repo-root> [<pristine-root>]
param(
    [Parameter(Mandatory=$true, Position=0)][string]$Repo,
    [Parameter(Position=1)][string]$Pristine
)
$ErrorActionPreference = 'Stop'
$Repo = (Resolve-Path -LiteralPath $Repo).Path
if (-not $Pristine) { $Pristine = Join-Path $Repo 'rust/target/astar-path-pristine' }
$Pristine = (Resolve-Path -LiteralPath $Pristine).Path
$Runner = Join-Path $PSScriptRoot 'run.ps1'
$Out = Join-Path $Repo 'rust/target/astar-path-diff/mutations'
New-Item -ItemType Directory -Force -Path $Out | Out-Null
$SourcePath = Join-Path $Repo 'rust/crates/hpl-ai/src/astar_path.rs'
$Original = [IO.File]::ReadAllBytes($SourcePath)
$Utf8 = [Text.UTF8Encoding]::new($false, $true)

$Mutants = @(
    @{ Name='drop-height-margin'; Edits=@(
       @{ Before='    let max_height = host.max_height() * 1.5;'; After='    let max_height = host.max_height();' }) },
    @{ Name='exclude-direct-height-boundary'; Edits=@(
       @{ Before='    if height <= max_height {'; After='    if height < max_height {' }) },
    @{ Name='direct-success-clears-search-sets'; Edits=@(
       @{ Before='            host.clear_goal();'; After='            host.reset_search();' }) },
    @{ Name='halve-candidate-radius'; Edits=@(
       @{ Before='    let max_distance = host.max_distance() * 2.0;'; After='    let max_distance = host.max_distance();' }) },
    @{ Name='include-candidate-distance-boundary'; Edits=@(
       @{ Before='            if distance < max_distance && height <= max_height {';
          After='            if distance <= max_distance && height <= max_height {' }) },
    @{ Name='refresh-captured-height-during-candidates'; Edits=@(
       @{ Before='            if distance < max_distance && height <= max_height {';
          After='            if distance < max_distance && height <= host.max_height() * 1.5 {' }) },
    @{ Name='recompute-start-distance-after-visibility'; Edits=@(
       @{ Before='                        Endpoint::Start => host.add_start(node, distance),';
          After='                        Endpoint::Start => host.add_start(node, math::vector3_dist(host.endpoint(endpoint), host.position(node))),' }) },
    @{ Name='double-precision-candidate-distance'; Edits=@(
       @{ Before='            let distance = math::vector3_dist(host.endpoint(endpoint), host.position(node));';
          After='            let distance = { let a = host.endpoint(endpoint); let b = host.position(node); let x = (a.x-b.x) as f64; let y = (a.y-b.y) as f64; let z = (a.z-b.z) as f64; ((x*x+y*y)+z*z).sqrt() as f32 };' }) },
    @{ Name='continue-after-visibility-exception'; Edits=@(
       @{ Before='                let Some(visible) = host.visible(endpoint, node) else { return false; };';
          After='                let Some(visible) = host.visible(endpoint, node) else { continue; };' }) },
    @{ Name='omit-goal-candidate-insertion'; Edits=@(
       @{ Before='                        Endpoint::Goal => host.add_goal(node),'; After='                        Endpoint::Goal => true,' }) },
    @{ Name='read-parent-before-output-append'; Edits=@(
       @{ Before="        loop {`n            if !host.append_path(search) { return false; }";
          After="        loop {`n            let cached_parent = host.parent(search);`n            if !host.append_path(search) { return false; }" },
       @{ Before='            let Some(parent) = host.parent(search) else { break; };'; After='            let Some(parent) = cached_parent else { break; };' }) },
    @{ Name='recheck-goal-after-output-callbacks'; Edits=@(
       @{ Before="    // the captured successful result still returns true.`n    true`n}";
          After="    // Mutant: recheck the goal modified by output callbacks.`n    host.goal_node().is_some()`n}" }) }
)
$Caught = 0
$Summary = [Collections.Generic.List[string]]::new()
try {
    & $Runner -Repo $Repo -Pristine $Pristine *> "$Out/baseline.log"
    foreach ($Mutant in $Mutants) {
        $Log = Join-Path $Out "$($Mutant.Name).log"
        $Source = $Utf8.GetString($Original)
        foreach ($Edit in $Mutant.Edits) {
            $Occurrences = [regex]::Matches($Source, [regex]::Escape($Edit.Before)).Count
            if ($Occurrences -ne 1) { throw "Expected one source anchor for $($Mutant.Name), found $Occurrences." }
            $Source = $Source.Replace($Edit.Before, $Edit.After)
        }
        $Failure = $null
        try {
            [IO.File]::WriteAllBytes($SourcePath, $Utf8.GetBytes($Source))
            try { & $Runner -Repo $Repo -Pristine $Pristine -Variant rust_release *> $Log }
            catch { $Failure = $_.Exception.Message; Add-Content -LiteralPath $Log -Value $Failure }
        } finally {
            [IO.File]::WriteAllBytes($SourcePath, $Original)
        }
        $Output = [IO.File]::ReadAllText($Log)
        $Difference = [regex]::Match($Output, 'First difference at line ([0-9]+):')
        if ($Failure -ne 'DIFFERENT: original != rust_release' -or -not $Difference.Success) {
            if (-not $Failure) { throw "SURVIVED: $($Mutant.Name). Extend the public API corpus before accepting this port." }
            throw "Invalid mutation result for $($Mutant.Name): $Failure. See $Log; build/runtime failures do not count."
        }
        ++$Caught
        $Result = "CAUGHT $($Mutant.Name): first difference at line $($Difference.Groups[1].Value)"
        $Summary.Add($Result)
        Write-Output $Result
    }
} finally {
    [IO.File]::WriteAllBytes($SourcePath, $Original)
    # Rebuild unmodified release/debug libraries even if a mutant survives.
    & $Runner -Repo $Repo -Pristine $Pristine *> "$Out/restored.log"
}

$Final = "$Caught/$($Mutants.Count) caught; 0 equivalent mutants; restored 4-way identical"
$Summary.Add($Final)
[IO.File]::WriteAllLines((Join-Path $Out 'summary.txt'), $Summary, $Utf8)
Write-Output $Final
