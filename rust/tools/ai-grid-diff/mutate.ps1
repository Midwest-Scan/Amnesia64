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
if (-not $Pristine) { $Pristine = Join-Path $Repo 'rust/target/ai-grid-pristine' }
$Pristine = (Resolve-Path -LiteralPath $Pristine).Path
$Runner = Join-Path $PSScriptRoot 'run.ps1'
$Out = Join-Path $Repo 'rust/target/ai-grid-diff/mutations'
New-Item -ItemType Directory -Force -Path $Out | Out-Null
$SourcePath = Join-Path $Repo 'rust/crates/hpl-ai/src/node_container.rs'
$Original = [IO.File]::ReadAllBytes($SourcePath)
$Utf8 = [Text.UTF8Encoding]::new($false, $true)

$Mutants = @(
    @{ Name='square-rounded-edge-distance'; Edits=@(
       @{ Before='(distance_squared(start, end).sqrt(), distance_squared(start, end))';
          After='(distance_squared(start, end).sqrt(), distance_squared(start, end).sqrt() * distance_squared(start, end).sqrt())' }) },
    @{ Name='double-precision-edge-sum'; Edits=@(
       @{ Before='else { (xx + yy) + zz }'; After='else { ((xx as f64 + yy as f64) + zz as f64) as f32 }' }) },
    @{ Name='omit-grid-rounding'; Edits=@(
       @{ Before='let rounded = (node_count as f32 / nodes_per_grid as f32).sqrt() + 0.5;';
          After='let rounded = (node_count as f32 / nodes_per_grid as f32).sqrt();' }) },
    @{ Name='extra-grid-border'; Edits=@(
       @{ Before='let side = dimension.checked_add(1).expect("AI grid border overflow");';
          After='let side = dimension.checked_add(2).expect("AI grid border overflow");' }) },
    @{ Name='double-cell-width'; Edits=@(
       @{ Before='Vector2f::new(extent.x / dimensions.map_size.x as f32, extent.y / dimensions.map_size.y as f32)';
          After='Vector2f::new(2.0 * (extent.x / dimensions.map_size.x as f32), extent.y / dimensions.map_size.y as f32)' }) },
    @{ Name='drop-insertion-row'; Edits=@(
       @{ Before='if cell.y > 0.0 { grid.y = f32_to_i32(local.y / cell.y); }'; After='if cell.y > 0.0 { grid.y = 0; }' }) },
    @{ Name='exclusive-query-border'; Edits=@(
       @{ Before='if grid.x > map_size.x { grid.x = map_size.x; }';
          After='if grid.x >= map_size.x { grid.x = map_size.x - 1; }' }) },
    @{ Name='omit-border-in-stride'; Edits=@(
       @{ Before='let stride = map_size.x.checked_add(1).expect("AI grid stride overflow");'; After='let stride = map_size.x;' }) },
    @{ Name='skip-final-grid-column'; Edits=@(
       @{ Before='if position.x > end.x {'; After='if position.x >= end.x {' }) },
    @{ Name='restart-each-row-at-zero'; Edits=@(
       @{ Before='position.x = start.x;'; After='position.x = 0;' }) },
    @{ Name='cache-node-count-before-resize'; Edits=@(
       @{ Before='    host.set_bounds(minimum, maximum);';
          After="    let cached_node_count = host.node_count();`n    host.set_bounds(minimum, maximum);" },
       @{ Before="    let mut i = 0;`n    while i < host.node_count() {";
          After="    let mut i = 0;`n    while i < cached_node_count {" }) },
    @{ Name='continue-after-push-exception'; Edits=@(
       @{ Before='if !host.push_node(index, node) { return; }'; After='host.push_node(index, node);' }) }
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
