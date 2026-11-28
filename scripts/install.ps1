$ErrorActionPreference = "Stop"

$Repo = "chaungoclong/draco"
$InstallDir = if ($env:DRACO_INSTALL_DIR) { $env:DRACO_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA "Draco\bin" }

if ($env:DRACO_VERSION) {
    $Tag = $env:DRACO_VERSION
} else {
    $Release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
    $Tag = $Release.tag_name
}

if (-not $Tag) {
    throw "Could not determine latest Draco release tag."
}

$Archive = "draco-$Tag-x86_64-windows.zip"
$BaseUrl = "https://github.com/$Repo/releases/download/$Tag"
$TmpDir = Join-Path ([System.IO.Path]::GetTempPath()) ("draco-install-" + [System.Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Force -Path $TmpDir | Out-Null

try {
    $ZipPath = Join-Path $TmpDir $Archive
    Write-Host "Downloading Draco $Tag (x86_64-windows)..."
    Invoke-WebRequest -Uri "$BaseUrl/$Archive" -OutFile $ZipPath

    # Verify SHA256 when SHA256SUMS.txt is present
    try {
        $SumsPath = Join-Path $TmpDir "SHA256SUMS.txt"
        Invoke-WebRequest -Uri "$BaseUrl/SHA256SUMS.txt" -OutFile $SumsPath
        $Line = Get-Content $SumsPath | Where-Object { $_ -match "\s+$Archive$" } | Select-Object -First 1
        if ($Line) {
            $Expected = ($Line -split "\s+")[0].ToLowerInvariant()
            $Actual = (Get-FileHash -Path $ZipPath -Algorithm SHA256).Hash.ToLowerInvariant()
            if ($Expected -ne $Actual) {
                throw "SHA256 checksum mismatch for $Archive!"
            }
            Write-Host "Checksum verified (SHA256)."
        }
    } catch {
        if ($_.Exception.Message -like "*mismatch*") { throw }
    }

    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    Expand-Archive -Path $ZipPath -DestinationPath $InstallDir -Force

    # Add to User PATH if missing
    $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if (-not ($UserPath -split ";" | Where-Object { $_ -eq $InstallDir })) {
        $NewPath = if ($UserPath) { "$UserPath;$InstallDir" } else { $InstallDir }
        [Environment]::SetEnvironmentVariable("Path", $NewPath, "User")
        Write-Host "Added $InstallDir to your User PATH (restart terminal to apply)."
    }

    Write-Host ""
    Write-Host "Draco $Tag installed to: $(Join-Path $InstallDir 'draco.exe')"
    Write-Host "Run: draco ."
} finally {
    Remove-Item -Recurse -Force $TmpDir -ErrorAction SilentlyContinue
}
