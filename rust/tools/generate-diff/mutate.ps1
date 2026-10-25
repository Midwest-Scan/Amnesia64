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
if (-not $Pristine) { $Pristine = Join-Path $Repo 'rust/target/generate-pristine' }
$Pristine = (Resolve-Path -LiteralPath $Pristine).Path
$Runner = Join-Path $PSScriptRoot 'run.ps1'
$Out = Join-Path $Repo 'rust/target/generate-diff/mutations'
New-Item -ItemType Directory -Force -Path $Out | Out-Null
$SourcePath = Join-Path $Repo 'rust/crates/hpl-generate/src/voxel_map.rs'
$Original = [IO.File]::ReadAllBytes($SourcePath)
$Utf8 = [Text.UTF8Encoding]::new($false, $true)

$Center = @'
let center = Vector3f::new(add_position(position.x, offset.x), add_position(position.y, offset.y),
                        add_position(position.z, offset.z)) + Vector3f::splat(half);
'@
$Center = $Center.Replace("`r`n", "`n")
$Reassociated = @'
let center = position + (Vector3f::new(x as f32, y as f32, z as f32) + Vector3f::splat(0.5)) * voxel_size;
'@
$Wide = @'
let center = Vector3f::new(
                        ((position.x as f64 + x as f64 * voxel_size as f64) + voxel_size as f64 / 2.0) as f32,
                        ((position.y as f64 + y as f64 * voxel_size as f64) + voxel_size as f64 / 2.0) as f32,
                        ((position.z as f64 + z as f64 * voxel_size as f64) + voxel_size as f64 / 2.0) as f32);
'@

$Mutants = @(
    @{ Name='allocation-one-extra-byte'; Edits=@(
       @{ Before='count as usize'; After='count as usize + 1' }) },
    @{ Name='mask-stored-byte'; Edits=@(
       @{ Before='data[linear_index(size, pos)] = value;'; After='data[linear_index(size, pos)] = value & 1;' }) },
    @{ Name='draw-any-nonzero-byte'; Edits=@(
       @{ Before='if host.byte(index) == 1 {'; After='if host.byte(index) != 0 {' }) },
    @{ Name='skip-alternate-x'; Edits=@(
       @{ Before='x += 1;'; After='x += 2;' }) },
    @{ Name='swap-render-x-y'; Edits=@(
       @{ Before='let offset = Vector3f::new(x as f32, y as f32, z as f32) * voxel_size;';
          After='let offset = Vector3f::new(y as f32, x as f32, z as f32) * voxel_size;' }) },
    @{ Name='full-size-radius'; Edits=@(
       @{ Before='if !host.draw_sphere(center, half) { return; }';
          After='if !host.draw_sphere(center, voxel_size) { return; }' }) },
    @{ Name='omit-center-half-offset'; Edits=@(
       @{ Before=' + Vector3f::splat(half);'; After=';' }) },
    @{ Name='reassociate-center'; Edits=@(
       @{ Before=$Center; After=$Reassociated.Replace("`r`n", "`n") }) },
    @{ Name='double-precision-center'; Edits=@(
       @{ Before=$Center; After=$Wide.Replace("`r`n", "`n") }) },
    @{ Name='cache-position-before-callbacks'; Edits=@(
       @{ Before='    let mut z = 0;'; After="    let cached_position = host.position();`n    let mut z = 0;" },
       @{ Before='let position = host.position();'; After='let position = cached_position;' }) },
    @{ Name='cache-voxel-size-before-callbacks'; Edits=@(
       @{ Before='    let mut z = 0;'; After="    let cached_voxel_size = host.voxel_size();`n    let mut z = 0;" },
       @{ Before='let voxel_size = host.voxel_size();'; After='let voxel_size = cached_voxel_size;' }) },
    @{ Name='ignore-draw-exception-stop'; Edits=@(
       @{ Before='if !host.draw_sphere(center, half) { return; }'; After='host.draw_sphere(center, half);' }) }
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
