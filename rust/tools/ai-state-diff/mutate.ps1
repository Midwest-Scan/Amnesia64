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
if (-not $Pristine) { $Pristine = Join-Path $Repo 'rust/target/ai-state-pristine' }
$Pristine = (Resolve-Path -LiteralPath $Pristine).Path
$Runner = Join-Path $PSScriptRoot 'run.ps1'
$Out = Join-Path $Repo 'rust/target/ai-state-diff/mutations'
New-Item -ItemType Directory -Force -Path $Out | Out-Null
$SourcePath = Join-Path $Repo 'rust/crates/hpl-ai/src/state_machine.rs'
$Original = [IO.File]::ReadAllBytes($SourcePath)
$Utf8 = [Text.UTF8Encoding]::new($false, $true)

$Mutants = @(
    @{ Name='remove-self-transition-guard'; Edits=@(
       @{ Before='    if id == host.state_id(current) { return; }'; After='    // Mutant: run callbacks for self transitions.' }) },
    @{ Name='drop-missing-state-warning'; Edits=@(
       @{ Before='        host.warn_missing(id);'; After='        // Mutant: omit the diagnostic callback.' }) },
    @{ Name='leave-uses-map-key-instead-of-live-target-id'; Edits=@(
       @{ Before='        let next_id = host.state_id(target);'; After='        let next_id = id;' }) },
    @{ Name='omit-leave-callback'; Edits=@(
       @{ Before='        if !host.leave(current, next_id) { return; }'; After='        // Mutant: omit OnLeaveState.' }) },
    @{ Name='cache-previous-id-before-leave'; Edits=@(
       @{ Before='    if let Some(current) = host.current() {'; After="    let cached_previous_id = host.state_id(current);`n    if let Some(current) = host.current() {" },
       @{ Before='    let previous_id = host.current().map_or(-1, |state| host.state_id(state));'; After='    let previous_id = cached_previous_id;' }) },
    @{ Name='enter-receives-destination-id'; Edits=@(
       @{ Before='    let previous_id = host.current().map_or(-1, |state| host.state_id(state));'; After='    let previous_id = host.state_id(target);' }) },
    @{ Name='continue-after-leave-exception'; Edits=@(
       @{ Before='        if !host.leave(current, next_id) { return; }'; After='        host.leave(current, next_id);' }) },
    @{ Name='commit-after-enter-exception'; Edits=@(
       @{ Before='    if !host.enter(target, previous_id) { return; }'; After='    host.enter(target, previous_id);' }) },
    @{ Name='assign-current-before-enter'; Edits=@(
       @{ Before='    if !host.enter(target, previous_id) { return; }'; After="    host.set_current(target);`n    if !host.enter(target, previous_id) { return; }" },
       @{ Before="    // Faithful: outer assignment overwrites a nested transition from OnEnter.`n    host.set_current(target);";
          After='    // Mutant: early assignment only.' }) },
    @{ Name='omit-final-current-assignment'; Edits=@(
       @{ Before='    host.set_current(target);'; After='    // Mutant: preserve the old or nested current state.' }) },
    @{ Name='invert-active-dispatch-gate'; Edits=@(
       @{ Before='    if !host.active() { return; }'; After='    if host.active() { return; }' }) },
    @{ Name='drop-update-elapsed-time'; Edits=@(
       @{ Before='    host.update(current, time);'; After='    host.update(current, 0.0);' }) }
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
