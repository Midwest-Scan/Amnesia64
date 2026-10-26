# Native Windows x64 AI navigation differential test (phase 2 public API).
# Baseline: git archive e91d65c HPL2/core rust/crates/hpl2-ffi/include.
# stubs.cpp uses a VirtualAlloc arena so pointer ties follow allocation order.
# Usage: ./run.ps1 <repo-root> [<pristine-root>]
# Fast mutation rerun: ./run.ps1 <repo-root> -Variant rust_release
# -SkipCargo reuses an already built Rust library; source files always rebuild.
param(
    [Parameter(Mandatory=$true, Position=0)][string]$Repo,
    [Parameter(Position=1)][string]$Pristine,
    [ValidateSet('all','original','cpp_switch_off','rust_release','rust_debug')][string]$Variant = 'all',
    [switch]$SkipCargo
)
$ErrorActionPreference = 'Stop'
$Repo = (Resolve-Path -LiteralPath $Repo).Path
if (-not $Pristine) { $Pristine = Join-Path $Repo 'rust/target/ai-nav-pristine' }
$Pristine = (Resolve-Path -LiteralPath $Pristine).Path
$Here = $PSScriptRoot
$Out = Join-Path $Repo 'rust/target/ai-nav-diff'
$Rust = Join-Path $Repo 'rust'
$Triple = 'x86_64-pc-windows-msvc'
New-Item -ItemType Directory -Force -Path $Out | Out-Null

# Load Microsoft's x64 compiler environment without changing VS/project settings.
if (-not (Get-Command cl.exe -ErrorAction SilentlyContinue) -or $env:VSCMD_ARG_TGT_ARCH -ne 'x64') {
    $VsWhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
    $VsRoot = & $VsWhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if (-not $VsRoot) { throw 'Visual Studio C++ x64 tools were not found.' }
    $DevCmd = Join-Path $VsRoot 'Common7/Tools/VsDevCmd.bat'
    $EnvironmentLines = & $env:ComSpec /d /s /c "`"$DevCmd`" -no_logo -arch=x64 -host_arch=x64 >nul && set"
    if ($LASTEXITCODE -ne 0) { throw 'Could not initialize the Visual Studio x64 environment.' }
    foreach ($Line in $EnvironmentLines) {
        if ($Line -match '^([^=]+)=(.*)$') { [Environment]::SetEnvironmentVariable($Matches[1], $Matches[2], 'Process') }
    }
}

$Variants = if ($Variant -eq 'all') { @('original','cpp_switch_off','rust_release','rust_debug') } else { @($Variant) }
if (-not $SkipCargo) {
    Push-Location $Rust
    try {
        foreach ($BuildVariant in $Variants) {
            if ($BuildVariant -notlike 'rust_*') { continue }
            $CargoArgs = @('build','-q','--locked','-p','hpl2-ffi','--target',$Triple)
            if ($BuildVariant -eq 'rust_release') { $CargoArgs += '--release' }
            & cargo @CargoArgs
            if ($LASTEXITCODE -ne 0) { throw "cargo failed for $BuildVariant" }
        }
    } finally { Pop-Location }
}

foreach ($BuildVariant in $Variants) {
    $SourceRoot = if ($BuildVariant -eq 'original') { $Pristine } else { $Repo }
    $Core = Join-Path $SourceRoot 'HPL2/core'
    # The original variant uses the pristine generated header when the baseline
    # includes rust/crates/hpl2-ffi/include (later ports remove types from it).
    $FfiInclude = Join-Path $Repo 'rust/crates/hpl2-ffi/include'
    $PristineFfi = Join-Path $Pristine 'rust/crates/hpl2-ffi/include'
    if ($BuildVariant -eq 'original' -and (Test-Path -LiteralPath $PristineFfi)) { $FfiInclude = $PristineFfi }
    $BuildDir = Join-Path $Out $BuildVariant
    New-Item -ItemType Directory -Force -Path $BuildDir | Out-Null
    $CompileArgs = @('/nologo','/std:c++17','/EHsc','/O2','/MD','/fp:precise','/utf-8','/W0','/Gy',
        '/D_CRT_SECURE_NO_WARNINGS',"/I$Core/include", "/I$FfiInclude")
    # Keep both native references independent of the linked Rust library.
    if ($BuildVariant -in @('original','cpp_switch_off')) { $CompileArgs += '/DHPL2_RUST_AI=0' }
    if ($BuildVariant -like 'rust_*') { $CompileArgs += @('/DHPL2_RUST_AI=1','/DHARNESS_RUST=1') }
    $CompileArgs += @("$Core/sources/ai/AINodeContainer.cpp", "$Core/sources/ai/AStar.cpp",
        "$Core/sources/impl/tinyXml/tinyxml.cpp", "$Core/sources/impl/tinyXml/tinyxmlparser.cpp",
        "$Core/sources/impl/tinyXml/tinyxmlerror.cpp", "$Core/sources/graphics/Color.cpp",
        "$Here/stubs.cpp", "$Here/diff_main.cpp")
    if ($BuildVariant -like 'rust_*') {
        $CompileArgs += @("$Core/sources/ai/AINodeContainerRust.cpp", "$Core/sources/ai/AStarRust.cpp",
            "$Core/sources/system/RustBridge.cpp")
    }
    $CompileArgs += @('/Fediff.exe','/link','/INCREMENTAL:NO','/OPT:REF')
    if ($BuildVariant -like 'rust_*') {
        $Profile = if ($BuildVariant -eq 'rust_release') { 'release' } else { 'debug' }
        $CompileArgs += "$Rust/target/$Triple/$Profile/hpl2_rs.lib"
    }
    Push-Location $BuildDir
    try {
        & cl.exe @CompileArgs *> compile.log
        if ($LASTEXITCODE -ne 0) { Get-Content compile.log; throw "C++ build failed for $BuildVariant" }
        # The fixture writes/reads a real .nodes file. A short relative path
        # also fits its fixed-size filename buffer regardless of repo location.
        $PreviousTemp = $env:NAV_DIFF_TMP
        try {
            $env:NAV_DIFF_TMP = 'save'
            & ./diff.exe "$Out/$BuildVariant.txt"
            if ($LASTEXITCODE -ne 0) { throw "Test failed for $BuildVariant (exit $LASTEXITCODE)" }
        } finally { $env:NAV_DIFF_TMP = $PreviousTemp }
    } finally { Pop-Location }
    $LineCount = ([IO.File]::ReadLines("$Out/$BuildVariant.txt") | Measure-Object -Line).Lines
    Write-Output "${BuildVariant}: $LineCount lines"
}

# Comparing SHA-256 of the complete binary files also checks line endings/logs.
$Reference = Join-Path $Out 'original.txt'
if (-not (Test-Path -LiteralPath $Reference)) { throw 'Run original (or all variants) first to establish the pristine reference.' }
$ReferenceHash = (Get-FileHash -LiteralPath $Reference -Algorithm SHA256).Hash
foreach ($BuildVariant in $Variants) {
    if ($BuildVariant -eq 'original') { continue }
    $Candidate = Join-Path $Out "$BuildVariant.txt"
    if ((Get-FileHash -LiteralPath $Candidate -Algorithm SHA256).Hash -ne $ReferenceHash) {
        $Left = [IO.File]::ReadAllLines($Reference)
        $Right = [IO.File]::ReadAllLines($Candidate)
        for ($Index=0; $Index -lt [Math]::Min($Left.Length, $Right.Length); ++$Index) {
            if ($Left[$Index] -cne $Right[$Index]) {
                Write-Output "First difference at line $($Index+1):"
                Write-Output "original: $($Left[$Index])"
                Write-Output "${BuildVariant}: $($Right[$Index])"
                break
            }
        }
        throw "DIFFERENT: original != $BuildVariant"
    }
    Write-Output "IDENTICAL: original == $BuildVariant"
}
if ($Variant -eq 'all') { Write-Output '4-way identical' }
