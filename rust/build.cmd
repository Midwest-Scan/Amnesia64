@echo off
rem Builds hpl2_rs.lib for Visual Studio (called by HPL2Rust.vcxproj).
rem Usage: build.cmd build|rebuild|clean <Configuration> <Platform>
setlocal

set ACTION=%~1
set CONFIG=%~2
set PLATFORM=%~3
set ROOT=%~dp0

if /I "%PLATFORM%"=="x64" (
    set TRIPLE=x86_64-pc-windows-msvc
) else if /I "%PLATFORM%"=="Win32" (
    set TRIPLE=i686-pc-windows-msvc
) else (
    echo %ROOT%build.cmd : error HPLRUST1: unsupported platform "%PLATFORM%"
    exit /b 1
)

if /I "%CONFIG%"=="Release" (
    set PROFILE=--release
) else (
    set PROFILE=
)

rem Visual Studio may have been started before rustup updated PATH.
where cargo >nul 2>nul
if errorlevel 1 if exist "%USERPROFILE%\.cargo\bin\cargo.exe" set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
where cargo >nul 2>nul
if errorlevel 1 (
    echo %ROOT%build.cmd : error HPLRUST2: cargo was not found. Install Rust from https://rustup.rs ^(default MSVC toolchain^), then restart Visual Studio.
    exit /b 1
)

pushd "%ROOT%"

if /I "%TRIPLE%"=="i686-pc-windows-msvc" (
    rustup target add %TRIPLE% >nul 2>nul
)

if /I "%ACTION%"=="clean" goto clean
if /I "%ACTION%"=="rebuild" (
    cargo clean --target %TRIPLE% %PROFILE%
)

cargo build -p hpl2-ffi --target %TRIPLE% %PROFILE%
set RESULT=%ERRORLEVEL%
popd
if not "%RESULT%"=="0" (
    echo %ROOT%build.cmd : error HPLRUST3: cargo build failed, see the output above.
)
exit /b %RESULT%

:clean
cargo clean --target %TRIPLE% %PROFILE%
set RESULT=%ERRORLEVEL%
popd
exit /b %RESULT%
