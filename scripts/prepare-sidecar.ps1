# prepare-sidecar.ps1
# Builds a complete Python environment for bundling into the YOLO Voice installer.
# Run this before `cargo tauri build` (or let CI run it).
#
# Result:
#   sidecar/python-env/   - Python 3.12.8 embeddable + pip + all deps
#   sidecar/models/       - Pre-downloaded tiny whisper model

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"  # Speed up Invoke-WebRequest

$ProjectRoot = Split-Path -Parent $PSScriptRoot
$SidecarDir  = Join-Path $ProjectRoot "sidecar"
$PythonEnvDir = Join-Path $SidecarDir "python-env"
$ModelsDir    = Join-Path $SidecarDir "models"
$TempDir      = Join-Path $SidecarDir "_build_temp"

$PythonVersion = "3.12.8"
$PythonZipUrl  = "https://www.python.org/ftp/python/$PythonVersion/python-$PythonVersion-embed-amd64.zip"
$GetPipUrl     = "https://bootstrap.pypa.io/get-pip.py"

# ── Guard: skip if already built ──────────────────────────────────────────────
if (Test-Path (Join-Path $PythonEnvDir "python.exe")) {
    Write-Host "[prepare-sidecar] Python environment already exists at $PythonEnvDir - skipping."
    Write-Host "[prepare-sidecar] Delete sidecar/python-env/ to force a rebuild."
    exit 0
}

Write-Host "[prepare-sidecar] Building Python sidecar environment..."
Write-Host "[prepare-sidecar] Python version: $PythonVersion"

# ── Clean slate ───────────────────────────────────────────────────────────────
if (Test-Path $PythonEnvDir) { Remove-Item -Recurse -Force $PythonEnvDir }
if (Test-Path $TempDir)      { Remove-Item -Recurse -Force $TempDir }
New-Item -ItemType Directory -Path $PythonEnvDir -Force | Out-Null
New-Item -ItemType Directory -Path $TempDir -Force | Out-Null

# ── Step 1: Download Python embeddable ────────────────────────────────────────
Write-Host "[prepare-sidecar] Step 1/5: Downloading Python $PythonVersion embeddable..."
$zipPath = Join-Path $TempDir "python-embed.zip"
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
Invoke-WebRequest -Uri $PythonZipUrl -OutFile $zipPath -UseBasicParsing

# ── Step 2: Extract ───────────────────────────────────────────────────────────
Write-Host "[prepare-sidecar] Step 2/5: Extracting Python..."
Expand-Archive -Path $zipPath -DestinationPath $PythonEnvDir -Force

# Enable pip: uncomment "import site" in python312._pth
$pthFile = Join-Path $PythonEnvDir "python312._pth"
if (Test-Path $pthFile) {
    (Get-Content $pthFile) -replace '^#import site', 'import site' | Set-Content $pthFile
    Write-Host "[prepare-sidecar]   Enabled 'import site' in python312._pth"
}

# ── Step 3: Install pip ──────────────────────────────────────────────────────
Write-Host "[prepare-sidecar] Step 3/5: Installing pip..."
$getPipPath = Join-Path $TempDir "get-pip.py"
Invoke-WebRequest -Uri $GetPipUrl -OutFile $getPipPath -UseBasicParsing

$pythonExe = Join-Path $PythonEnvDir "python.exe"
& $pythonExe $getPipPath --no-warn-script-location 2>&1 | Out-Host
if ($LASTEXITCODE -ne 0) { throw "get-pip.py failed with exit code $LASTEXITCODE" }

# ── Step 4: Install dependencies ─────────────────────────────────────────────
Write-Host "[prepare-sidecar] Step 4/5: Installing Python dependencies..."
$requirementsPath = Join-Path $SidecarDir "requirements.txt"
& $pythonExe -m pip install -r $requirementsPath --no-warn-script-location 2>&1 | Out-Host
if ($LASTEXITCODE -ne 0) { throw "pip install failed with exit code $LASTEXITCODE" }

# ── Step 5: Download tiny whisper model ───────────────────────────────────────
Write-Host "[prepare-sidecar] Step 5/5: Downloading tiny whisper model..."
New-Item -ItemType Directory -Path $ModelsDir -Force | Out-Null
$modelDir = Join-Path $ModelsDir "faster-whisper-tiny"

if (!(Test-Path $modelDir)) {
    & $pythonExe -c @"
from huggingface_hub import snapshot_download
snapshot_download('Systran/faster-whisper-tiny', local_dir=r'$modelDir')
print('Model downloaded successfully')
"@ 2>&1 | Out-Host
    if ($LASTEXITCODE -ne 0) { throw "Model download failed with exit code $LASTEXITCODE" }
} else {
    Write-Host "[prepare-sidecar]   Model already exists, skipping download."
}

# ── Cleanup ───────────────────────────────────────────────────────────────────
Write-Host "[prepare-sidecar] Cleaning up temp files and pip cache..."
Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue

# Clear pip cache to reduce size
& $pythonExe -m pip cache purge 2>&1 | Out-Null

# Remove unnecessary files to shrink bundle
$removePatterns = @("__pycache__", "*.dist-info", "tests", "test")
# Keep __pycache__ removal light — only remove test directories
Get-ChildItem -Path $PythonEnvDir -Recurse -Directory -Filter "tests" | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
Get-ChildItem -Path $PythonEnvDir -Recurse -Directory -Filter "test" | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue

# ── Summary ───────────────────────────────────────────────────────────────────
$envSize = (Get-ChildItem -Path $PythonEnvDir -Recurse | Measure-Object -Property Length -Sum).Sum / 1MB
$modelSize = (Get-ChildItem -Path $ModelsDir -Recurse | Measure-Object -Property Length -Sum).Sum / 1MB
Write-Host ""
Write-Host "[prepare-sidecar] Done!"
Write-Host "[prepare-sidecar]   Python env size: $([math]::Round($envSize, 1)) MB"
Write-Host "[prepare-sidecar]   Model size:      $([math]::Round($modelSize, 1)) MB"
Write-Host "[prepare-sidecar]   Total:           $([math]::Round($envSize + $modelSize, 1)) MB"
