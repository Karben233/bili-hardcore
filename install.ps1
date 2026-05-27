#Requires -Version 5.1
$ErrorActionPreference = "Stop"

$REPO = "Karben233/bili-hardcore"
$BINARY = "bili-hardcore.exe"
$InstallDir = if ($env:INSTALL_DIR) { $env:INSTALL_DIR } else { "$env:LOCALAPPDATA\Programs\bili-hardcore" }

# --- Get latest version ---
Write-Host "[info] 正在获取最新版本..." -ForegroundColor Blue
$release = Invoke-RestMethod -Uri "https://api.github.com/repos/$REPO/releases/latest"
$version = $release.tag_name
if (-not $version) { Write-Host "[error] 无法获取最新版本号" -ForegroundColor Red; exit 1 }
Write-Host "[info] 最新版本: $version" -ForegroundColor Blue

# --- Download ---
$filename = "bili-hardcore-$version-windows-x64.zip"
$url = "https://github.com/$REPO/releases/download/$version/$filename"
$tmpdir = "$env:TEMP\bili-hardcore-install"
New-Item -ItemType Directory -Force -Path $tmpdir | Out-Null

Write-Host "[info] 正在下载 $filename..." -ForegroundColor Blue
$zipPath = "$tmpdir\$filename"
Invoke-WebRequest -Uri $url -OutFile $zipPath -UseBasicParsing

# --- Install ---
Write-Host "[info] 正在解压..." -ForegroundColor Blue
Expand-Archive -Path $zipPath -DestinationPath $tmpdir -Force

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Move-Item -Path "$tmpdir\$BINARY" -Destination "$InstallDir\$BINARY" -Force

# --- Add to PATH (user level) ---
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$userPath;$InstallDir", "User")
    $env:Path += ";$InstallDir"
    Write-Host "[info] 已添加 $InstallDir 到用户 PATH（新终端窗口生效）" -ForegroundColor Yellow
}

# --- Cleanup ---
Remove-Item -Recurse -Force $tmpdir

Write-Host "[info] 安装完成: $InstallDir\$BINARY" -ForegroundColor Green
Write-Host "[info] 运行 'bili-hardcore --help' 开始使用" -ForegroundColor Green
