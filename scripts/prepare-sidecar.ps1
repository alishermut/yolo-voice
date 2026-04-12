# prepare-sidecar.ps1
# Builds the bundled Python runtime used by the Distil-Whisper sidecar.
#
# Result:
#   sidecar/python-env/ - Python 3.12.8 embeddable + CUDA-enabled Distil deps

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$ProjectRoot = Split-Path -Parent $PSScriptRoot
$SidecarDir = Join-Path $ProjectRoot "sidecar"
$PythonEnvDir = Join-Path $SidecarDir "python-env"
$TempDir = Join-Path $SidecarDir "_build_temp"

$PythonVersion = "3.12.8"
$PythonZipUrl = "https://www.python.org/ftp/python/$PythonVersion/python-$PythonVersion-embed-amd64.zip"
$GetPipUrl = "https://bootstrap.pypa.io/get-pip.py"
$TorchVersion = "2.6.0"
$TorchWheelIndexUrl = "https://download.pytorch.org/whl/cu124"
$ExpectedTorchCudaVersionPrefix = "12.4"

function Get-TorchProbe {
    param(
        [string]$PythonExe
    )

    if (!(Test-Path $PythonExe)) {
        return $null
    }

    $probeOutput = & $PythonExe -c @'
import json
try:
    import torch
    print(json.dumps({
        'torch_version': getattr(torch, '__version__', ''),
        'cuda_version': getattr(getattr(torch, 'version', None), 'cuda', None),
        'cuda_available': bool(torch.cuda.is_available()),
        'device_count': int(torch.cuda.device_count()),
    }))
except Exception as exc:
    print(json.dumps({'error': str(exc)}))
    raise
'@ 2>$null

    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($probeOutput)) {
        return $null
    }

    try {
        return $probeOutput | ConvertFrom-Json
    } catch {
        return $null
    }
}

function Remove-PathIfExists {
    param(
        [string]$Path
    )

    if (Test-Path $Path) {
        Remove-Item -LiteralPath $Path -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Prune-PythonEnv {
    param(
        [string]$Root
    )

    $sitePackages = Join-Path $Root "Lib\site-packages"
    $torchDir = Join-Path $sitePackages "torch"

    foreach ($path in @(
        (Join-Path $Root "Scripts"),
        (Join-Path $Root "Include"),
        (Join-Path $sitePackages "pip"),
        (Join-Path $sitePackages "pip-26.0.1.dist-info"),
        (Join-Path $sitePackages "setuptools"),
        (Join-Path $sitePackages "setuptools-70.2.0.dist-info"),
        (Join-Path $sitePackages "pkg_resources"),
        (Join-Path $torchDir "include"),
        (Join-Path $torchDir "lib\cusolverMg64_11.dll"),
        (Join-Path $torchDir "lib\curand64_10.dll"),
        (Join-Path $torchDir "lib\cupti64_2024.1.0.dll"),
        (Join-Path $torchDir "lib\cufftw64_11.dll"),
        (Join-Path $torchDir "lib\cudnn_cnn64_9.dll"),
        (Join-Path $torchDir "lib\cudnn_adv64_9.dll")
    )) {
        Remove-PathIfExists $path
    }

    Get-ChildItem -Path $Root -Recurse -Directory -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -in @("__pycache__", "tests", "test") } |
        ForEach-Object { Remove-PathIfExists $_.FullName }

    Get-ChildItem -Path $Root -Recurse -File -Include *.pyc,*.pyo,*.lib -ErrorAction SilentlyContinue |
        Remove-Item -Force -ErrorAction SilentlyContinue
}

$existingPythonExe = Join-Path $PythonEnvDir "python.exe"
if (Test-Path $existingPythonExe) {
    $existingTorchProbe = Get-TorchProbe $existingPythonExe
    if (
        $existingTorchProbe -and
        $existingTorchProbe.cuda_version -and
        $existingTorchProbe.cuda_version.StartsWith($ExpectedTorchCudaVersionPrefix)
    ) {
        Write-Host "[prepare-sidecar] CUDA-enabled Python environment already exists at $PythonEnvDir - skipping."
        Write-Host "[prepare-sidecar] Existing torch: $($existingTorchProbe.torch_version) (CUDA $($existingTorchProbe.cuda_version))"
        exit 0
    }

    Write-Host "[prepare-sidecar] Existing Python environment is missing the expected CUDA-enabled PyTorch runtime - rebuilding."
}

Write-Host "[prepare-sidecar] Building Python sidecar environment..."
Write-Host "[prepare-sidecar] Python version: $PythonVersion"

Remove-PathIfExists $PythonEnvDir
Remove-PathIfExists $TempDir
Remove-PathIfExists (Join-Path $SidecarDir "models")
New-Item -ItemType Directory -Path $PythonEnvDir -Force | Out-Null
New-Item -ItemType Directory -Path $TempDir -Force | Out-Null

Write-Host "[prepare-sidecar] Step 1/6: Downloading Python $PythonVersion embeddable..."
$zipPath = Join-Path $TempDir "python-embed.zip"
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
Invoke-WebRequest -Uri $PythonZipUrl -OutFile $zipPath -UseBasicParsing

Write-Host "[prepare-sidecar] Step 2/6: Extracting Python..."
Expand-Archive -Path $zipPath -DestinationPath $PythonEnvDir -Force

$pthFile = Join-Path $PythonEnvDir "python312._pth"
if (Test-Path $pthFile) {
    (Get-Content $pthFile) -replace '^#import site', 'import site' | Set-Content $pthFile
    Write-Host "[prepare-sidecar]   Enabled 'import site' in python312._pth"
}

Write-Host "[prepare-sidecar] Step 3/6: Installing pip..."
$getPipPath = Join-Path $TempDir "get-pip.py"
Invoke-WebRequest -Uri $GetPipUrl -OutFile $getPipPath -UseBasicParsing

$pythonExe = Join-Path $PythonEnvDir "python.exe"
& $pythonExe $getPipPath --no-warn-script-location 2>&1 | Out-Host
if ($LASTEXITCODE -ne 0) { throw "get-pip.py failed with exit code $LASTEXITCODE" }

Write-Host "[prepare-sidecar] Step 4/6: Installing CUDA-enabled PyTorch..."
& $pythonExe -m pip install "torch==$TorchVersion" --index-url $TorchWheelIndexUrl --no-warn-script-location 2>&1 | Out-Host
if ($LASTEXITCODE -ne 0) { throw "CUDA torch install failed with exit code $LASTEXITCODE" }

Write-Host "[prepare-sidecar] Step 5/6: Installing Python dependencies..."
$requirementsPath = Join-Path $SidecarDir "requirements.txt"
& $pythonExe -m pip install -r $requirementsPath --no-warn-script-location 2>&1 | Out-Host
if ($LASTEXITCODE -ne 0) { throw "pip install failed with exit code $LASTEXITCODE" }

$torchProbe = Get-TorchProbe $pythonExe
if (
    -not $torchProbe -or
    -not $torchProbe.cuda_version -or
    -not $torchProbe.cuda_version.StartsWith($ExpectedTorchCudaVersionPrefix)
) {
    $observedTorchVersion = if ($torchProbe) { $torchProbe.torch_version } else { "unknown" }
    $observedCudaVersion = if ($torchProbe) { $torchProbe.cuda_version } else { "none" }
    throw "Expected a CUDA-enabled torch build ($ExpectedTorchCudaVersionPrefix.x) but found torch=$observedTorchVersion cuda=$observedCudaVersion"
}

Write-Host "[prepare-sidecar]   Torch: $($torchProbe.torch_version)"
Write-Host "[prepare-sidecar]   CUDA runtime in wheel: $($torchProbe.cuda_version)"
Write-Host "[prepare-sidecar]   CUDA available on build machine: $($torchProbe.cuda_available)"
Write-Host "[prepare-sidecar]   Visible CUDA devices on build machine: $($torchProbe.device_count)"

Write-Host "[prepare-sidecar] Step 6/6: Cleaning up pip cache and pruning non-runtime files..."
& $pythonExe -m pip cache purge 2>&1 | Out-Null
Prune-PythonEnv $PythonEnvDir
Remove-PathIfExists $TempDir

$envSize = (Get-ChildItem -Path $PythonEnvDir -Recurse | Measure-Object -Property Length -Sum).Sum / 1MB
Write-Host ""
Write-Host "[prepare-sidecar] Done!"
Write-Host "[prepare-sidecar]   Python env size: $([math]::Round($envSize, 1)) MB"
Write-Host "[prepare-sidecar]   Total bundled sidecar size: $([math]::Round($envSize, 1)) MB"
