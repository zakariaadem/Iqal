# Builds dist\iqal-setup-<version>.exe: the release programs, the website
# lists and the Mozilla-signed Firefox extension, carried inside the setup
# program (see crates\iqal-setup\build.rs).
#
#   powershell -File packaging\windows\build-setup.ps1            (everything)
#   ... -Stage programs   only the programs, into target\setup-payload
#   ... -Stage setup -Payload <folder>   the setup around a payload folder
#
# The automatic build signs the programs between the two stages, since
# Windows checks the signature of every program it starts, not only the
# setup's.
param([ValidateSet("all", "programs", "setup")] [string]$Stage = "all", [string]$Payload = "")
$ErrorActionPreference = "Stop"
$root = Resolve-Path "$PSScriptRoot\..\.."
Set-Location $root
if (-not $Payload) { $Payload = Join-Path $root "target\setup-payload" }

if ($Stage -ne "setup") {
    cargo build --release --locked -p iqal-daemon -p iqal-cli -p iqal-host -p iqal-agent -p iqal-win32
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    if (Test-Path $Payload) { Remove-Item -Recurse -Force $Payload }
    New-Item -ItemType Directory -Force "$Payload\lists", "$Payload\extension" | Out-Null
    foreach ($exe in "iqal-daemon", "iqal-cli", "iqal-nm-host", "iqal-agent", "iqal-win32") {
        Copy-Item "target\release\$exe.exe" $Payload
    }
    Copy-Item "lists\*.txt" "$Payload\lists"
    # Firefox only accepts extensions signed by Mozilla; Chrome and Edge
    # install theirs from their stores.
    Copy-Item "packaging\windows\iqal-firefox-signed.xpi" "$Payload\extension\iqal.xpi"
    "programs in $Payload"
}

if ($Stage -ne "programs") {
    $env:IQAL_SETUP_PAYLOAD = (Resolve-Path $Payload).Path
    cargo build --release --locked -p iqal-setup
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $version = (Select-String -Path Cargo.toml -Pattern '^version = "(.+)"').Matches[0].Groups[1].Value
    New-Item -ItemType Directory -Force dist | Out-Null
    $out = "dist\iqal-setup-$version.exe"
    Copy-Item "target\release\iqal-setup.exe" $out -Force
    "built $out ($([math]::Round((Get-Item $out).Length / 1MB, 1)) MB)"
}
