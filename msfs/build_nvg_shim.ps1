param(
    [Parameter(Mandatory = $true)]
    [string]$ProjectRoot,

    [Parameter(Mandatory = $true)]
    [string]$OutDir
)

$ErrorActionPreference = 'Stop'

$shimRoot = Join-Path $ProjectRoot 'nvg_shim'
if (-not (Test-Path $shimRoot)) {
    throw "nvg_shim folder not found at: $shimRoot"
}

$buildDir = Join-Path $OutDir 'nvg_shim_build'
New-Item -ItemType Directory -Force -Path $buildDir | Out-Null

# Configure + build.
# We intentionally build Release to keep the DLL small/fast.
#
# Important: don't build with LLVM clang++ 17 here. Newer MSVC STL versions hard-require
# Clang 19+ when using clang-cl/clang++ with MSVC headers.
# Using the MSVC generator forces cl.exe, which is fine for this shim.
$generator = "Visual Studio 17 2022"
$arch = "x64"

& cmake -S $shimRoot -B $buildDir -G $generator -A $arch | Out-Null
& cmake --build $buildDir --config Release | Out-Null

# Locate the produced DLL.
$dll = Get-ChildItem -Path $buildDir -Recurse -Filter 'nvg_shim.dll' -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $dll) {
    $dll = Get-ChildItem -Path $buildDir -Recurse -Filter '*shim*.dll' -ErrorAction SilentlyContinue | Select-Object -First 1
}
if (-not $dll) {
    throw "Could not find shim DLL under: $buildDir"
}

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
Copy-Item -Force -Path $dll.FullName -Destination (Join-Path $OutDir 'nanovg_shim.dll')

Write-Host "Built shim: $($dll.FullName) -> $(Join-Path $OutDir 'nanovg_shim.dll')"
