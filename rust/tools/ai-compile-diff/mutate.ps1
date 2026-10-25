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
if (-not $Pristine) { $Pristine = Join-Path $Repo 'rust/target/ai-compile-pristine' }
$Pristine = (Resolve-Path -LiteralPath $Pristine).Path
$Runner = Join-Path $PSScriptRoot 'run.ps1'
$Out = Join-Path $Repo 'rust/target/ai-compile-diff/mutations'
New-Item -ItemType Directory -Force -Path $Out | Out-Null
$SourcePath = Join-Path $Repo 'rust/crates/hpl-ai/src/node_compile.rs'
$Original = [IO.File]::ReadAllBytes($SourcePath)
$Utf8 = [Text.UTF8Encoding]::new($false, $true)

$Mutants = @(
    @{ Name='accept-self-candidates'; Edits=@(
       @{ Before='        if candidate == source { continue; }'; After='        // Mutant: accept self candidates.' }) },
    @{ Name='halve-candidate-cutoff'; Edits=@(
       @{ Before='if distance > host.max_distance() * 2.0 { continue; }';
          After='if distance > host.max_distance() { continue; }' }) },
    @{ Name='exclude-distance-boundary'; Edits=@(
       @{ Before='if distance > host.max_distance() * 2.0 { continue; }';
          After='if distance >= host.max_distance() * 2.0 { continue; }' }) },
    @{ Name='signed-height-difference'; Edits=@(
       @{ Before='let height = (host.position(source).y - host.position(candidate).y).abs();';
          After='let height = host.position(source).y - host.position(candidate).y;' }) },
    @{ Name='exclude-height-boundary'; Edits=@(
       @{ Before='        if height <= host.max_height() {'; After='        if height < host.max_height() {' }) },
    @{ Name='accept-blocked-paths'; Edits=@(
       @{ Before='if visible && !host.add_edge(candidate) { return; }';
          After='if !host.add_edge(candidate) { return; }' }) },
    @{ Name='continue-after-edge-exception'; Edits=@(
       @{ Before='if visible && !host.add_edge(candidate) { return; }';
          After='if visible { host.add_edge(candidate); }' }) },
    @{ Name='zero-cap-removes-all'; Edits=@(
       @{ Before='if maximum > 0 && (host.edge_count() as i32) > maximum && !host.resize_edges(maximum as usize) { return; }';
          After='if maximum >= 0 && (host.edge_count() as i32) > maximum && !host.resize_edges(maximum as usize) { return; }' }) },
    @{ Name='keep-one-extra-minimum-edge'; Edits=@(
       @{ Before='if host.edge_distance(index) > host.max_distance() && (index as i32) >= host.min_edges() {';
          After='if host.edge_distance(index) > host.max_distance() && (index as i32) > host.min_edges() {' }) },
    @{ Name='omit-native-sort'; Edits=@(
       @{ Before='    if !host.sort_edges() { return; }'; After='    // Mutant: leave insertion order.' }) },
    @{ Name='cache-distance-limit-before-visibility'; Edits=@(
       @{ Before='pub fn compile_node<H: CompileHost>(host: &mut H, source: H::Node) {';
          After="pub fn compile_node<H: CompileHost>(host: &mut H, source: H::Node) {`n    let cached_maximum_distance = host.max_distance();" },
       @{ Before='if distance > host.max_distance() * 2.0 { continue; }';
          After='if distance > cached_maximum_distance * 2.0 { continue; }' },
       @{ Before='if host.edge_distance(index) > host.max_distance() && (index as i32) >= host.min_edges() {';
          After='if host.edge_distance(index) > cached_maximum_distance && (index as i32) >= host.min_edges() {' }) },
    @{ Name='double-precision-distance'; Edits=@(
       @{ Before='let distance = math::vector3_dist(host.position(source), host.position(candidate));';
          After='let distance = { let a = host.position(source); let b = host.position(candidate); let x = (a.x-b.x) as f64; let y = (a.y-b.y) as f64; let z = (a.z-b.z) as f64; ((x*x+y*y)+z*z).sqrt() as f32 };' }) }
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
