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
if (-not $Pristine) { $Pristine = Join-Path $Repo 'rust/target/ai-scheduler-pristine' }
$Pristine = (Resolve-Path -LiteralPath $Pristine).Path
$Runner = Join-Path $PSScriptRoot 'run.ps1'
$Out = Join-Path $Repo 'rust/target/ai-scheduler-diff/mutations'
New-Item -ItemType Directory -Force -Path $Out | Out-Null
$SourcePath = Join-Path $Repo 'rust/crates/hpl-ai/src/state_machine.rs'
$Original = [IO.File]::ReadAllBytes($SourcePath)
$Utf8 = [Text.UTF8Encoding]::new($false, $true)

$Mutants = @(
    @{ Name='drop-elapsed-time'; Edits=@(
       @{ Before='    host.set_time_count(host.time_count() + time);'; After='    // Mutant: drop elapsed time.' }) },
    @{ Name='halve-elapsed-time'; Edits=@(
       @{ Before='    host.set_time_count(host.time_count() + time);'; After='    host.set_time_count(host.time_count() + time * 0.5);' }) },
    @{ Name='exclude-equal-tick-boundary'; Edits=@(
       @{ Before='    while host.time_count() >= host.update_step() {'; After='    while host.time_count() > host.update_step() {' }) },
    @{ Name='overwrite-nested-counter-changes'; Edits=@(
       @{ Before='        if !host.on_update(host.update_step()) { return; }';
          After='        let old_count = host.time_count(); if !host.on_update(host.update_step()) { return; }' },
       @{ Before='        host.set_time_count(host.time_count() - host.update_step());';
          After='        host.set_time_count(old_count - host.update_step());' }) },
    @{ Name='cap-scheduler-at-one-tick'; Edits=@(
       @{ Before='    while host.time_count() >= host.update_step() {'; After='    if host.time_count() >= host.update_step() {' }) },
    @{ Name='callback-receives-elapsed-time'; Edits=@(
       @{ Before='        if !host.on_update(host.update_step()) { return; }'; After='        if !host.on_update(time) { return; }' }) },
    @{ Name='subtract-despite-callback-exception'; Edits=@(
       @{ Before='        if !host.on_update(host.update_step()) { return; }';
          After='        if !host.on_update(host.update_step()) { host.set_time_count(host.time_count() - host.update_step()); return; }' }) },
    @{ Name='reject-nonpositive-steps'; Edits=@(
       @{ Before='    while host.time_count() >= host.update_step() {';
          After="    if host.update_step() <= 0.0 { return; }`n    while host.time_count() >= host.update_step() {" }) },
    @{ Name='subtract-absolute-step'; Edits=@(
       @{ Before='        host.set_time_count(host.time_count() - host.update_step());';
          After='        host.set_time_count(host.time_count() - host.update_step().abs());' }) },
    @{ Name='retain-double-precision-accumulator'; Edits=@(
       @{ Before='    host.set_time_count(host.time_count() + time);';
          After='    let mut count = host.time_count() as f64 + time as f64; host.set_time_count(count as f32);' },
       @{ Before='    while host.time_count() >= host.update_step() {';
          After='    while count >= host.update_step() as f64 {' },
       @{ Before='        host.set_time_count(host.time_count() - host.update_step());';
          After='        count -= host.update_step() as f64; host.set_time_count(count as f32);' }) }
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
            $Candidate = Join-Path $Repo 'rust/target/ai-scheduler-diff/rust_release.txt'
            if (Test-Path -LiteralPath $Candidate) { Copy-Item -LiteralPath $Candidate -Destination (Join-Path $Out "$($Mutant.Name)-output.txt") }
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
