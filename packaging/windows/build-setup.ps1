# Builds dist\iqal-setup-<version>.exe: the release programs, the website
# lists and the Mozilla-signed Firefox extension, carried inside the setup
# program (see crates\iqal-setup\build.rs).
#
# Usage, from the repository: powershell -File packaging\windows\build-setup.ps1
$ErrorActionPreference = "Stop"
$root = Resolve-Path "$PSScriptRoot\..\.."
Set-Location $root

cargo build --release --locked -p iqal-daemon -p iqal-cli -p iqal-host -p iqal-agent -p iqal-win32
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$payload = Join-Path $root "target\setup-payload"
if (Test-Path $payload) { Remove-Item -Recurse -Force $payload }
New-Item -ItemType Directory -Force "$payload\lists", "$payload\extension" | Out-Null
foreach ($exe in "iqal-daemon", "iqal-cli", "iqal-nm-host", "iqal-agent", "iqal-win32") {
    Copy-Item "target\release\$exe.exe" $payload
}
Copy-Item "lists\*.txt" "$payload\lists"
# Firefox only accepts extensions signed by Mozilla; Chrome and Edge
# install theirs from their stores.
Copy-Item "packaging\windows\iqal-firefox-signed.xpi" "$payload\extension\iqal.xpi"

$env:IQAL_SETUP_PAYLOAD = $payload
cargo build --release --locked -p iqal-setup
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$version = (Select-String -Path Cargo.toml -Pattern '^version = "(.+)"').Matches[0].Groups[1].Value
New-Item -ItemType Directory -Force dist | Out-Null
$out = "dist\iqal-setup-$version.exe"
Copy-Item "target\release\iqal-setup.exe" $out -Force
"built $out ($([math]::Round((Get-Item $out).Length / 1MB, 1)) MB)"
