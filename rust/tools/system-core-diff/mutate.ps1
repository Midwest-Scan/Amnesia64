# Plant one realistic Rust bug at a time and require the public C++ API
# differential test to detect it. Compile failures do not count as catches.
# Source bytes are restored after every mutant and again on every exit path.
# Do not edit/build these Rust sources concurrently with this script.
# Usage: ./mutate.ps1 <repo-root> [<pristine-root>]
param(
    [Parameter(Mandatory=$true, Position=0)][string]$Repo,
    [Parameter(Position=1)][string]$Pristine
)
$ErrorActionPreference = 'Stop'
$Repo = (Resolve-Path -LiteralPath $Repo).Path
if (-not $Pristine) { $Pristine = Join-Path $Repo 'rust/target/system-core-pristine' }
$Pristine = (Resolve-Path -LiteralPath $Pristine).Path
$Runner = Join-Path $PSScriptRoot 'run.ps1'
$Out = Join-Path $Repo 'rust/target/system-core-diff/mutations'
New-Item -ItemType Directory -Force -Path $Out | Out-Null
$Timer = Join-Path $Repo 'rust/crates/hpl-system/src/logic_timer.rs'
$Platform = Join-Path $Repo 'rust/crates/hpl-system/src/platform.rs'
$Original = @{}
foreach ($Path in @($Timer, $Platform)) { $Original[$Path] = [IO.File]::ReadAllBytes($Path) }
$Utf8 = [Text.UTF8Encoding]::new($false, $true)

$Mutants = @(
    @{ Name='initial-cap-divisor'; Path=$Timer;
       Before='max_updates: updates_per_sec / 10,'; After='max_updates: updates_per_sec / 9,' },
    @{ Name='double-counter-increment'; Path=$Timer;
       Before='self.update_count = self.update_count.wrapping_add(1);'; After='self.update_count = self.update_count.wrapping_add(2);' },
    @{ Name='reject-at-cap'; Path=$Timer;
       Before='if self.update_count > self.max_updates { return false; }'; After='if self.update_count >= self.max_updates { return false; }' },
    @{ Name='update-at-clock-equality'; Path=$Timer;
       Before='if self.local_time < clock() {'; After='if self.local_time <= clock() {' },
    @{ Name='reset-at-cap'; Path=$Timer;
       Before="if self.update_count > self.max_updates {`n            self.reset(clock);";
       After="if self.update_count >= self.max_updates {`n            self.reset(clock);" },
    @{ Name='omit-loop-counter-reset'; Path=$Timer;
       Before='self.update_count = 0;'; After='// Mutation: omit the loop counter reset.' },
    @{ Name='rate-change-recomputes-cap'; Path=$Timer;
       Before='self.local_time_add = 1000.0 / (updates_per_sec as f64);';
       After="self.local_time_add = 1000.0 / (updates_per_sec as f64);`n        self.max_updates = updates_per_sec / 10;" },
    @{ Name='step-narrows-after-division'; Path=$Timer;
       Before='(self.local_time_add as f32) / 1000.0_f32'; After='(self.local_time_add / 1000.0) as f32' },
    @{ Name='multiply-speed'; Path=$Timer;
       Before='self.local_time += self.local_time_add / self.speed_mul;'; After='self.local_time += self.local_time_add * self.speed_mul;' },
    @{ Name='single-precision-increment'; Path=$Timer;
       Before='self.local_time += self.local_time_add / self.speed_mul;';
       After='self.local_time += ((self.local_time_add as f32) / (self.speed_mul as f32)) as f64;' },
    @{ Name='reverse-width-order'; Path=$Platform;
       Before='if size_a.x != size_b.x { return size_a.x < size_b.x; }';
       After='if size_a.x != size_b.x { return size_a.x > size_b.x; }' },
    @{ Name='omit-height-key'; Path=$Platform;
       Before='if size_a.y != size_b.y { return size_a.y < size_b.y; }';
       After='// Mutation: omit the height comparison.' }
)

$Caught = 0
$Summary = [Collections.Generic.List[string]]::new()
try {
    # Establish a fresh reference and make sure the unmodified port passes.
    & $Runner -Repo $Repo -Pristine $Pristine *> "$Out/baseline.log"
    foreach ($Mutant in $Mutants) {
        $Log = Join-Path $Out "$($Mutant.Name).log"
        $Source = $Utf8.GetString($Original[$Mutant.Path])
        $Occurrences = [regex]::Matches($Source, [regex]::Escape($Mutant.Before)).Count
        if ($Occurrences -ne 1) { throw "Expected one source anchor for $($Mutant.Name), found $Occurrences." }
        $Mutated = $Source.Replace($Mutant.Before, $Mutant.After)
        $Failure = $null
        try {
            [IO.File]::WriteAllBytes($Mutant.Path, $Utf8.GetBytes($Mutated))
            try { & $Runner -Repo $Repo -Pristine $Pristine -Variant rust_release *> $Log }
            catch { $Failure = $_.Exception.Message; Add-Content -LiteralPath $Log -Value $Failure }
        } finally {
            [IO.File]::WriteAllBytes($Mutant.Path, $Original[$Mutant.Path])
        }
        $Output = [IO.File]::ReadAllText($Log)
        if ($Failure -ne 'DIFFERENT: original != rust_release' -or $Output -notmatch 'First difference at line ([0-9]+):') {
            if (-not $Failure) { throw "SURVIVED: $($Mutant.Name). Add a public API test case before accepting this port." }
            throw "Invalid mutation result for $($Mutant.Name): $Failure. See $Log; build/runtime failures do not count."
        }
        $Line = $Matches[1]
        ++$Caught
        $Result = "CAUGHT $($Mutant.Name): first difference at line $Line"
        $Summary.Add($Result)
        Write-Output $Result
    }
} finally {
    foreach ($Path in @($Timer, $Platform)) { [IO.File]::WriteAllBytes($Path, $Original[$Path]) }
}

# Leave both the source tree and built libraries at the original implementation.
& $Runner -Repo $Repo -Pristine $Pristine *> "$Out/restored.log"
$Final = "$Caught/$($Mutants.Count) caught; 0 equivalent mutants; restored 4-way identical"
$Summary.Add($Final)
[IO.File]::WriteAllLines((Join-Path $Out 'summary.txt'), $Summary, $Utf8)
Write-Output $Final
