$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$ProjectRoot = Split-Path -Parent $PSScriptRoot
$SidecarDir = Join-Path $ProjectRoot "sidecar"
$PythonEnvDir = Join-Path $SidecarDir "python-env"
$TempDir = Join-Path $SidecarDir "_build_temp"

$PythonVersion = "3.12.8"
$PythonZipUrl = "https://www.python.org/ftp/python/$PythonVersion/python-$PythonVersion-embed-amd64.zip"
$GetPipUrl = "https://bootstrap.pypa.io/get-pip.py"
$ExpectedTorchVersionPrefix = "2.6.0"
$ExpectedTransformersVersion = "5.5.3"
$ExpectedHubVersion = "1.10.1"
$ExpectedSoundfileVersion = "0.13.1"

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
    import transformers
    import huggingface_hub
    import soundfile
    print(json.dumps({
        'torch_version': getattr(torch, '__version__', ''),
        'cuda_version': getattr(getattr(torch, 'version', None), 'cuda', None),
        'transformers_version': getattr(transformers, '__version__', ''),
        'huggingface_hub_version': getattr(huggingface_hub, '__version__', ''),
        'soundfile_version': getattr(soundfile, '__version__', ''),
    }))
except Exception as exc:
    print(json.dumps({'error': str(exc)}))
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

$existingPythonExe = Join-Path $PythonEnvDir "python.exe"
if (Test-Path $existingPythonExe) {
    $existingTorchProbe = Get-TorchProbe $existingPythonExe
    if (
        $existingTorchProbe -and
        $existingTorchProbe.torch_version -and
        $existingTorchProbe.torch_version.StartsWith($ExpectedTorchVersionPrefix) -and
        $existingTorchProbe.transformers_version -eq $ExpectedTransformersVersion -and
        $existingTorchProbe.huggingface_hub_version -eq $ExpectedHubVersion -and
        $existingTorchProbe.soundfile_version -eq $ExpectedSoundfileVersion -and
        !$existingTorchProbe.cuda_version
    ) {
        Write-Host "[prepare-sidecar] Base Python environment already exists at $PythonEnvDir - skipping."
        Write-Host "[prepare-sidecar] Existing torch: $($existingTorchProbe.torch_version)"
        exit 0
    }

    Write-Host "[prepare-sidecar] Existing Python environment does not match the expected bundled CPU runtime - rebuilding."
}

Write-Host "[prepare-sidecar] Building bundled Distil Python runtime..."
Write-Host "[prepare-sidecar] Python version: $PythonVersion"

Remove-PathIfExists $PythonEnvDir
Remove-PathIfExists $TempDir
New-Item -ItemType Directory -Path $PythonEnvDir -Force | Out-Null
New-Item -ItemType Directory -Path $TempDir -Force | Out-Null

Write-Host "[prepare-sidecar] Step 1/5: Downloading Python $PythonVersion embeddable..."
$zipPath = Join-Path $TempDir "python-embed.zip"
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
Invoke-WebRequest -Uri $PythonZipUrl -OutFile $zipPath -UseBasicParsing

Write-Host "[prepare-sidecar] Step 2/5: Extracting Python..."
Expand-Archive -Path $zipPath -DestinationPath $PythonEnvDir -Force

$pthFile = Join-Path $PythonEnvDir "python312._pth"
if (Test-Path $pthFile) {
    (Get-Content $pthFile) -replace '^#import site', 'import site' | Set-Content $pthFile
}

Write-Host "[prepare-sidecar] Step 3/5: Installing pip..."
$getPipPath = Join-Path $TempDir "get-pip.py"
Invoke-WebRequest -Uri $GetPipUrl -OutFile $getPipPath -UseBasicParsing

$pythonExe = Join-Path $PythonEnvDir "python.exe"
& $pythonExe $getPipPath --no-warn-script-location 2>&1 | Out-Host
if ($LASTEXITCODE -ne 0) { throw "get-pip.py failed with exit code $LASTEXITCODE" }

Write-Host "[prepare-sidecar] Step 4/5: Installing base Distil dependencies..."
$requirementsPath = Join-Path $SidecarDir "requirements.txt"
& $pythonExe -m pip install -r $requirementsPath --no-warn-script-location 2>&1 | Out-Host
if ($LASTEXITCODE -ne 0) { throw "pip install failed with exit code $LASTEXITCODE" }

Write-Host "[prepare-sidecar] Step 5/5: Pruning temp files..."
& $pythonExe -m pip cache purge 2>&1 | Out-Null
Get-ChildItem -Path $PythonEnvDir -Recurse -Directory -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -in @("__pycache__", "tests", "test") } |
    ForEach-Object { Remove-PathIfExists $_.FullName }
Get-ChildItem -Path $PythonEnvDir -Recurse -File -Include *.pyc,*.pyo -ErrorAction SilentlyContinue |
    Remove-Item -Force -ErrorAction SilentlyContinue
Remove-PathIfExists $TempDir

$torchProbe = Get-TorchProbe $pythonExe
if (
    -not $torchProbe -or
    -not $torchProbe.torch_version -or
    !$torchProbe.torch_version.StartsWith($ExpectedTorchVersionPrefix) -or
    $torchProbe.transformers_version -ne $ExpectedTransformersVersion -or
    $torchProbe.huggingface_hub_version -ne $ExpectedHubVersion -or
    $torchProbe.soundfile_version -ne $ExpectedSoundfileVersion
) {
    throw "Bundled Distil runtime is missing the expected torch build."
}

if ($torchProbe.cuda_version) {
    throw "Bundled Distil runtime should remain CPU-based. Found CUDA build $($torchProbe.cuda_version)."
}

$envSize = (Get-ChildItem -Path $PythonEnvDir -Recurse | Measure-Object -Property Length -Sum).Sum / 1MB
Write-Host ""
Write-Host "[prepare-sidecar] Done!"
Write-Host "[prepare-sidecar]   Torch: $($torchProbe.torch_version)"
Write-Host "[prepare-sidecar]   Transformers: $($torchProbe.transformers_version)"
Write-Host "[prepare-sidecar]   Hugging Face Hub: $($torchProbe.huggingface_hub_version)"
Write-Host "[prepare-sidecar]   SoundFile: $($torchProbe.soundfile_version)"
Write-Host "[prepare-sidecar]   Python env size: $([math]::Round($envSize, 1)) MB"
