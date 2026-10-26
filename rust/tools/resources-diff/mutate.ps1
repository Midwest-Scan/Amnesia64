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
if (-not $Pristine) { $Pristine = Join-Path $Repo 'rust/target/resources-pristine' }
$Pristine = (Resolve-Path -LiteralPath $Pristine).Path
$Runner = Join-Path $PSScriptRoot 'run.ps1'
$Out = Join-Path $Repo 'rust/target/resources-diff/mutations'
New-Item -ItemType Directory -Force -Path $Out | Out-Null
$SourcePath = Join-Path $Repo 'rust/crates/hpl-resources/src/file_searcher.rs'
$Original = [IO.File]::ReadAllBytes($SourcePath)
$Utf8 = [Text.UTF8Encoding]::new($false, $true)

$Mutants = @(
    @{ Name='case-sensitive-query-key'; Edits=@(
       @{ Before='        let key = s::to_lower_case(&s::get_file_name(name));'; After='        let key = s::get_file_name(name);' }) },
    @{ Name='case-sensitive-insert-key'; Edits=@(
       @{ Before='            let key = s::to_lower_case(&s::to_8char(&file));'; After='            let key = s::to_8char(&file);' }) },
    @{ Name='keep-backslash-search-path'; Edits=@(
       @{ Before='        let path = slashes(path);'; After='        let path = path.to_vec();' }) },
    @{ Name='scan-only-first-file'; Edits=@(
       @{ Before='        for file in files {'; After='        for file in files.into_iter().take(1) {' }) },
    @{ Name='drop-recursion'; Edits=@(
       @{ Before='        if recurse {'; After='        if false && recurse {' }) },
    @{ Name='last-score-tie-wins'; Edits=@(
       @{ Before='            if count > best_count { best_count = count; best = entry.native; }'; After='            if count >= best_count { best_count = count; best = entry.native; }' }) },
    @{ Name='omit-wanted-subsequence'; Edits=@(
       @{ Before='            let count = reverse_matches(&wanted, &entry.dirs).max(reverse_matches(&entry.dirs, &wanted));'; After='            let count = reverse_matches(&entry.dirs, &wanted);' }) },
    @{ Name='omit-available-subsequence'; Edits=@(
       @{ Before='            let count = reverse_matches(&wanted, &entry.dirs).max(reverse_matches(&entry.dirs, &wanted));'; After='            let count = reverse_matches(&wanted, &entry.dirs);' }) },
    @{ Name='discard-stored-filename-component'; Edits=@(
       @{ Before='            let dirs = components(&full);'; After='            let mut dirs = components(&full); dirs.pop();' }) },
    @{ Name='unique-entry-skips-output-count'; Edits=@(
       @{ Before='        if entries.len() == 1 && !want_count { return (Some(first), None); }'; After='        if entries.len() == 1 { let _ = want_count; return (Some(first), None); }' }) },
    @{ Name='empty-wanted-path-writes-count'; Edits=@(
       @{ Before='        if path.is_empty() { return (Some(first), None); }'; After='        if path.is_empty() { return (Some(first), Some(0)); }' }) },
    @{ Name='ignore-duplicate-first-path'; Edits=@(
       @{ Before='            if duplicate { continue; }'; After='            if false && duplicate { continue; }' }) }
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
            $Candidate = Join-Path $Repo 'rust/target/resources-diff/rust_release.txt'
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
