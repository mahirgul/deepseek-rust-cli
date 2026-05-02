$repo = "mahirgul/deepseek-rust-cli"
$os = "windows-x86_64"

$latestRelease = (Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases/latest").tag_name

if (-not $latestRelease) {
    Write-Error "Could not find latest release for $repo"
    exit
}

$url = "https://github.com/$repo/releases/download/$latestRelease/deepseek-rust-cli-$os.zip"
$zipFile = "$env:TEMP\deepseek-rust-cli.zip"
$destFolder = "$env:USERPROFILE\.deepseek-cli\bin"

if (-not (Test-Path $destFolder)) {
    New-Item -Path $destFolder -ItemType Directory -Force
}

Write-Host "Downloading DeepSeek Rust CLI $latestRelease..."
Invoke-WebRequest -Uri $url -OutFile $zipFile

Expand-Archive -Path $zipFile -DestinationPath $destFolder -Force
Remove-Item $zipFile

$path = [Environment]::GetEnvironmentVariable("Path", "User")
if ($path -notlike "*$destFolder*") {
    [Environment]::SetEnvironmentVariable("Path", "$path;$destFolder", "User")
    $env:Path += ";$destFolder"
}

Write-Host "Successfully installed deepseek-rust-cli to $destFolder"
Write-Host "Please restart your terminal to start using 'deepseek-rust-cli.exe'"
