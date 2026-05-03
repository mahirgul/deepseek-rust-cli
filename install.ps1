# ─── DeepSeek Rust CLI - Windows Installer ────────────────────
# Usage: iwr https://.../install.ps1 -useb | iex

param (
    [switch]$SkipChecksum,
    [string]$InstallDir = "$env:USERPROFILE\.deepseek-cli\bin"
)

$repo = "mahirgul/deepseek-rust-cli"
$binName = "deepseek-rust-cli.exe"
$ErrorActionPreference = "Stop"

# ─── Color Helpers ─────────────────────────────────────────
function Write-Info    { Write-Host "[INFO]  $args" -ForegroundColor Cyan }
function Write-Success { Write-Host "[OK]    $args" -ForegroundColor Green }
function Write-Warn    { Write-Host "[WARN]  $args" -ForegroundColor Yellow }
function Write-Error   { Write-Host "[ERROR] $args" -ForegroundColor Red }

# ─── Detect Architecture ───────────────────────────────────
function Get-Platform {
    $arch = (Get-WmiObject Win32_Processor).Architecture
    if ($arch -eq 9) {
        # ARM64
        if ([Environment]::Is64BitOperatingSystem) {
            return "windows-arm64"
        }
    }
    if ([Environment]::Is64BitOperatingSystem) {
        return "windows-x86_64"
    }
    Write-Error "32-bit Windows is not supported"
    exit 1
}

# ─── Main ──────────────────────────────────────────────────
function Main {
    Write-Host ""
    Write-Host "╔══════════════════════════════════════════════╗" -ForegroundColor Cyan
    Write-Host "║   DeepSeek Rust CLI - Windows Installer      ║" -ForegroundColor Cyan
    Write-Host "╚══════════════════════════════════════════════╝" -ForegroundColor Cyan
    Write-Host ""

    $platform = Get-Platform
    Write-Info "Detected platform: $platform"

    # Get latest release
    Write-Info "Fetching latest release..."
    try {
        $latestRelease = (Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases/latest" -TimeoutSec 10).tag_name
    } catch {
        Write-Error "Could not fetch latest release: $_"
        exit 1
    }

    if (-not $latestRelease) {
        Write-Error "Could not find latest release for $repo"
        exit 1
    }
    Write-Info "Latest version: $latestRelease"

    # Download
    $archiveName = "$binName-$platform.zip" -replace '\.exe-', '-'
    $downloadUrl = "https://github.com/$repo/releases/download/$latestRelease/$archiveName"
    $checksumUrl = "$downloadUrl.sha256"

    $tempDir = Join-Path $env:TEMP "deepseek-rust-cli-install"
    New-Item -ItemType Directory -Force -Path $tempDir | Out-Null

    $zipFile = Join-Path $tempDir $archiveName

    Write-Info "Downloading $archiveName..."
    try {
        Invoke-WebRequest -Uri $downloadUrl -OutFile $zipFile -TimeoutSec 60
    } catch {
        Write-Error "Download failed: $_"
        exit 1
    }

    # Checksum verification
    if (-not $SkipChecksum) {
        try {
            $expectedSha = (Invoke-RestMethod -Uri $checksumUrl -TimeoutSec 10).Split(' ')[0]
            $actualSha = (Get-FileHash -Path $zipFile -Algorithm SHA256).Hash.ToLower()
            if ($actualSha -eq $expectedSha) {
                Write-Success "Checksum verified"
            } else {
                Write-Error "Checksum FAILED! Expected: $expectedSha, Got: $actualSha"
                Write-Warn "Use -SkipChecksum to bypass verification"
                exit 1
            }
        } catch {
            Write-Warn "Could not verify checksum: $_"
        }
    }

    # Extract
    Write-Info "Extracting..."
    Expand-Archive -Path $zipFile -DestinationPath $tempDir -Force

    # Install
    if (-not (Test-Path $InstallDir)) {
        New-Item -Path $InstallDir -ItemType Directory -Force | Out-Null
    }

    $sourceExe = Get-ChildItem -Path $tempDir -Filter "*.exe" -Recurse | Select-Object -First 1
    if (-not $sourceExe) {
        Write-Error "Could not find .exe in archive"
        exit 1
    }

    Copy-Item -Path $sourceExe.FullName -Destination $InstallDir -Force

    # Add to PATH
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -notlike "*$InstallDir*") {
        Write-Info "Adding to user PATH..."
        [Environment]::SetEnvironmentVariable("Path", "$userPath;$InstallDir", "User")
        $env:Path += ";$InstallDir"
        Write-Success "Added $InstallDir to PATH"
        Write-Warn "Please restart your terminal for PATH changes to take effect"
    }

    # Cleanup
    Remove-Item -Path $tempDir -Recurse -Force -ErrorAction SilentlyContinue

    Write-Success "Successfully installed deepseek-rust-cli to $InstallDir"
    Write-Host ""
    Write-Info "Run 'deepseek-rust-cli' to start"
    Write-Info "Make sure to set DEEPSEEK_API_KEY environment variable"

    # Offer to set API key
    $setKey = Read-Host "`nSet DEEPSEEK_API_KEY now? (y/N)"
    if ($setKey -eq 'y' -or $setKey -eq 'Y') {
        $apiKey = Read-Host "Enter your DeepSeek API key"
        if ($apiKey) {
            [Environment]::SetEnvironmentVariable("DEEPSEEK_API_KEY", $apiKey, "User")
            $env:DEEPSEEK_API_KEY = $apiKey
            Write-Success "API key set for current user"
        }
    }

    Write-Host ""
}

Main
