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
if (-not $Pristine) { $Pristine = Join-Path $Repo 'rust/target/astar-pristine' }
$Pristine = (Resolve-Path -LiteralPath $Pristine).Path
$Runner = Join-Path $PSScriptRoot 'run.ps1'
$Out = Join-Path $Repo 'rust/target/astar-diff/mutations'
New-Item -ItemType Directory -Force -Path $Out | Out-Null
$SourcePath = Join-Path $Repo 'rust/crates/hpl-ai/src/astar.rs'
$Original = [IO.File]::ReadAllBytes($SourcePath)
$Utf8 = [Text.UTF8Encoding]::new($false, $true)

$Mutants = @(
    @{ Name='last-pointer-wins-ties'; Edits=@(
       @{ Before='if best_cost > node_cost {'; After='if best_cost >= node_cost {' }) },
    @{ Name='select-highest-cost'; Edits=@(
       @{ Before='if best_cost > node_cost {'; After='if best_cost < node_cost {' }) },
    @{ Name='omit-height-absolute-value'; Edits=@(
       @{ Before='let height = 1.0 + (node_y - parent_y).abs();'; After='let height = 1.0 + (node_y - parent_y);' }) },
    @{ Name='omit-unit-height-cost'; Edits=@(
       @{ Before='let height = 1.0 + (node_y - parent_y).abs();'; After='let height = (node_y - parent_y).abs();' }) },
    @{ Name='double-precision-cost'; Edits=@(
       @{ Before='distance * height'; After='(distance as f64 * (1.0f64 + (node_y as f64 - parent_y as f64).abs())) as f32' }) },
    @{ Name='squared-heuristic'; Edits=@(
       @{ Before='.sqrt()'; After='' }) },
    @{ Name='one-extra-iteration'; Edits=@(
       @{ Before='if !(limit < 0 || iteration < limit) { break; }'; After='if !(limit < 0 || iteration <= limit) { break; }' }) },
    @{ Name='skip-first-edge'; Edits=@(
       @{ Before='for index in 0..count {'; After='for index in 1..count {' }) },
    @{ Name='cache-distance-before-callback'; Edits=@(
       @{ Before='            let Some(allowed) = host.can_add(node, child) else { return; };';
          After="            let distance = host.search_distance(search) + host.edge_distance(edge);`n            let Some(allowed) = host.can_add(node, child) else { return; };" },
       @{ Before="                let distance = host.search_distance(search) + host.edge_distance(edge);`n"; After='' }) },
    @{ Name='cache-iteration-limit'; Edits=@(
       @{ Before='    let mut iteration = 0i32;'; After="    let mut iteration = 0i32;`n    let limit = host.max_iterations();" },
       @{ Before="        let limit = host.max_iterations();`n"; After='' }) },
    @{ Name='cache-child-before-callback'; Edits=@(
       @{ Before="                let child = host.edge_node(edge);`n"; After='' }) },
    @{ Name='continue-after-callback-exception'; Edits=@(
       @{ Before='let Some(allowed) = host.can_add(node, child) else { return; };';
          After='let Some(allowed) = host.can_add(node, child) else { continue; };' }) }
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
