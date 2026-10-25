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
if (-not $Pristine) { $Pristine = Join-Path $Repo 'rust/target/ai-placement-pristine' }
$Pristine = (Resolve-Path -LiteralPath $Pristine).Path
$Runner = Join-Path $PSScriptRoot 'run.ps1'
$Out = Join-Path $Repo 'rust/target/ai-placement-diff/mutations'
New-Item -ItemType Directory -Force -Path $Out | Out-Null
$SourcePath = Join-Path $Repo 'rust/crates/hpl-ai/src/node_generator.rs'
$Original = [IO.File]::ReadAllBytes($SourcePath)
$Utf8 = [Text.UTF8Encoding]::new($false, $true)

$Mutants = @(
    @{ Name='expand-minimum-x-instead-of-shrink'; Edits=@(
       @{ Before='    minimum.x += host.grid_size();'; After='    minimum.x -= host.grid_size();' }) },
    @{ Name='double-precision-wall-correction'; Edits=@(
       @{ Before='                    host.set_position(node, host.position(node) + push);';
          After='                    let p = host.position(node); let amount = host.wall_distance() as f64 - distance as f64; let direction = directions[index]; host.set_position(node, Vector3f::new((p.x as f64 + direction.x as f64 * amount) as f32, (p.y as f64 + direction.y as f64 * amount) as f32, (p.z as f64 + direction.z as f64 * amount) as f32));' }) },
    @{ Name='exclude-last-grid-row'; Edits=@(
       @{ Before='    while position.z <= maximum.z {'; After='    while position.z < maximum.z {' }) },
    @{ Name='wrap-before-last-grid-column'; Edits=@(
       @{ Before='        if position.x > maximum.x {'; After='        if position.x >= maximum.x {' }) },
    @{ Name='step-from-live-generator-params'; Edits=@(
       @{ Before='        position.x += host.step_size();'; After='        position.x += host.grid_size();' }) },
    @{ Name='refresh-captured-wall-offsets'; Edits=@(
       @{ Before='            let hit = match host.cast_wall(node, ends[index]) { Ok(hit) => hit, Err(CallbackFailed) => return };';
          After='            let w = host.wall_distance(); let offset = match index { 0 => Vector3f::new(w, 0.0, 0.0), 1 => Vector3f::new(-w, 0.0, 0.0), 2 => Vector3f::new(0.0, 0.0, w), _ => Vector3f::new(0.0, 0.0, -w) }; let hit = match host.cast_wall(node, offset) { Ok(hit) => hit, Err(CallbackFailed) => return };' }) },
    @{ Name='include-equal-wall-distance'; Edits=@(
       @{ Before='                if distance < host.wall_distance() {'; After='                if distance <= host.wall_distance() {' }) },
    @{ Name='skip-zero-axis-vector-arithmetic'; Edits=@(
       @{ Before='                    host.set_position(node, host.position(node) + push);';
          After='                    let mut p = host.position(node); if index < 2 { p.x += push.x; } else { p.z += push.z; } host.set_position(node, p);' }) },
    @{ Name='snapshot-position-before-wall-ray'; Edits=@(
       @{ Before='            let hit = match host.cast_wall(node, ends[index]) { Ok(hit) => hit, Err(CallbackFailed) => return };';
          After='            let saved_position = host.position(node); let hit = match host.cast_wall(node, ends[index]) { Ok(hit) => hit, Err(CallbackFailed) => return };' },
       @{ Before='                    host.set_position(node, host.position(node) + push);'; After='                    host.set_position(node, saved_position + push);' }) },
    @{ Name='accept-negative-and-nan-mass'; Edits=@(
       @{ Before='    if mass != 0.0 { return true; }'; After='    if mass > 0.0 { return true; }' }) },
    @{ Name='subtract-ground-height'; Edits=@(
       @{ Before='    let position = host.point() + Vector3f::new(0.0, host.height(), 0.0);';
          After='    let position = host.point() + Vector3f::new(0.0, -host.height(), 0.0);' }) },
    @{ Name='continue-after-wall-ray-exception'; Edits=@(
       @{ Before='            let hit = match host.cast_wall(node, ends[index]) { Ok(hit) => hit, Err(CallbackFailed) => return };';
          After='            let hit = match host.cast_wall(node, ends[index]) { Ok(hit) => hit, Err(CallbackFailed) => continue };' }) }
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
