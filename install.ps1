# ─── DeepSeek Rust CLI - Windows Installer ────────────────────
# Usage: iwr https://.../install.ps1 -useb | iex

param (
    [switch]$SkipChecksum,
    [string]$InstallDir = "$env:USERPROFILE\.deepseek-cli\bin"
)

[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$OutputEncoding = [System.Text.Encoding]::UTF8
$ErrorActionPreference = "Stop"

$REPO      = "mahirgul/deepseek-rust-cli"
$BIN_NAME  = "deepseek-rust-cli"
$EXE_NAME  = "$BIN_NAME.exe"

# ─── Helpers ────────────────────────────────────────────────

function Write-Info    { Write-Host "[INFO]  $($args -join ' ')" -ForegroundColor Cyan   }
function Write-Success { Write-Host "[OK]    $($args -join ' ')" -ForegroundColor Green  }
function Write-Warn    { Write-Host "[WARN]  $($args -join ' ')" -ForegroundColor Yellow }
function Write-Error   { Write-Host "[ERROR] $($args -join ' ')" -ForegroundColor Red    }

function Get-Platform {
    $arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
    if ($arch -eq "Arm64") {
        return "windows-arm64"
    } elseif ($arch -eq "X64") {
        return "windows-x86_64"
    } else {
        Write-Error "Unsupported architecture: $arch"
        exit 1
    }
}

# ─── Main ──────────────────────────────────────────────────

function Main {
    Write-Host ""
    Write-Host "DeepSeek Rust CLI - Windows Installer" -ForegroundColor Cyan
    Write-Host ""

    $platform = Get-Platform
    Write-Info "Detected platform: $platform"

    # ── Resolve latest version ─────────────────────────
    Write-Info "Fetching latest release..."
    try {
        $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$REPO/releases/latest" -TimeoutSec 10
        $latestTag = $release.tag_name
    } catch {
        Write-Error "Could not fetch latest release: $_"
        exit 1
    }

    if (-not $latestTag) {
        Write-Error "No release found for $REPO"
        exit 1
    }
    Write-Info "Latest version: $latestTag"

    # ── Build URLs ────────────────────────────────────
    $archiveName  = "$BIN_NAME-$platform.zip"
    $downloadUrl  = "https://github.com/$REPO/releases/download/$latestTag/$archiveName"
    $checksumUrl  = "$downloadUrl.sha256"

    # ── Create temp workspace ─────────────────────────
    $tempDir = Join-Path $env:TEMP "deepseek-cli-$PID"
    New-Item -ItemType Directory -Force -Path $tempDir | Out-Null

    $zipFile = Join-Path $tempDir $archiveName

    # ── Download ──────────────────────────────────────
    Write-Info "Downloading $archiveName..."
    try {
        Invoke-WebRequest -Uri $downloadUrl -OutFile $zipFile -TimeoutSec 60
    } catch {
        Write-Error "Download failed: $_"
        Remove-Item -Recurse -Force $tempDir -ErrorAction SilentlyContinue
        exit 1
    }

    # ── Verify checksum ───────────────────────────────
    if (-not $SkipChecksum) {
        try {
            $expectedSha = (Invoke-RestMethod -Uri $checksumUrl -TimeoutSec 10).Split(' ')[0]
            $actualSha   = (Get-FileHash -Path $zipFile -Algorithm SHA256).Hash.ToLower()

            if ($actualSha -eq $expectedSha) {
                Write-Success "Checksum verified"
            } else {
                Write-Error "Checksum mismatch! Expected: $expectedSha, Got: $actualSha"
                Write-Warn "Re-run with -SkipChecksum to bypass verification (not recommended)"
                Remove-Item -Recurse -Force $tempDir -ErrorAction SilentlyContinue
                exit 1
            }
        } catch {
            Write-Warn "Could not verify checksum: $_"
        }
    } else {
        Write-Warn "Checksum verification skipped"
    }

    # ── Extract ───────────────────────────────────────
    Write-Info "Extracting..."
    Expand-Archive -Path $zipFile -DestinationPath $tempDir -Force

    # ── Locate binary in extracted contents ───────────
    $exe = Get-ChildItem -Path $tempDir -Filter $EXE_NAME -Recurse | Select-Object -First 1
    if (-not $exe) {
        Write-Error "Could not find $EXE_NAME in downloaded archive"
        Remove-Item -Recurse -Force $tempDir -ErrorAction SilentlyContinue
        exit 1
    }

    # ── Install ───────────────────────────────────────
    if (-not (Test-Path $InstallDir)) {
        New-Item -Path $InstallDir -ItemType Directory -Force | Out-Null
    }

    Copy-Item -Path $exe.FullName -Destination $InstallDir -Force
    Write-Success "Copied $EXE_NAME to $InstallDir"

    # ── Update PATH ───────────────────────────────────
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if (-not $userPath) { $userPath = "" }
    if ($userPath -notlike "*$InstallDir*") {
        Write-Info "Adding to user PATH..."
        [Environment]::SetEnvironmentVariable("Path", "$userPath;$InstallDir", "User")
        $env:Path = "$env:Path;$InstallDir"
        Write-Success "Added $InstallDir to PATH"
        Write-Warn "Please restart your terminal for PATH changes to take effect"
    }

    # ── Cleanup ───────────────────────────────────────
    Remove-Item -Recurse -Force $tempDir -ErrorAction SilentlyContinue

    Write-Success "deepseek-rust-cli installed successfully!"
    Write-Host ""
    Write-Info "Run 'deepseek-rust-cli' to start"
    Write-Info "Set DEEPSEEK_API_KEY environment variable before use"

    # ── Optional: set API key ─────────────────────────
    $setKey = Read-Host "`nSet DEEPSEEK_API_KEY now? (y/N)"
    if ($setKey -match '^[yY]') {
        $apiKey = Read-Host "Enter your DeepSeek API key"
        if ($apiKey) {
            [Environment]::SetEnvironmentVariable("DEEPSEEK_API_KEY", $apiKey, "User")
            $env:DEEPSEEK_API_KEY = $apiKey
            Write-Success "API key saved"
        }
    }

    Write-Host ""
}

Main
