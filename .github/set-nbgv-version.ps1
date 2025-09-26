# param(
#     [string]$nuGetPackageVersion = $(nbgv get-version -v NuGetPackageVersion).Trim()
# )

$semverVersion = $(nbgv get-version -v NuGetPackageVersion).Trim()
$fourpartVersion = $(nbgv get-version -v Version).Trim()

$originalLocation = Get-Location

# setting cargo workspace version
$scriptDir = "$PSScriptRoot/.."
$path = Join-Path $scriptDir "Cargo.toml"
Write-Host "Setting version to $semverVersion"

(Get-Content $path) | ForEach-Object {
    if ($_ -match '^version\s*=\s*".*"') {
        $_ -replace '^version\s*=\s*".*"', "version = `"$semverVersion`""
    }
    else {
        $_
    }
} | Set-Content $path

# nodejs version is injected by workflow during packaging; skip npm version bump here

# python version is injected by workflow during packaging; skip pyproject.toml rewrite here

# copying README.md
Copy-Item -Path "$scriptDir/README_NUGET.md" -Destination "$scriptDir/src/lib-nodejs/README.md" -Force
Copy-Item -Path "$scriptDir/README_NUGET.md" -Destination "$scriptDir/src/lib-rust/README.md" -Force
Copy-Item -Path "$scriptDir/README_NUGET.md" -Destination "$scriptDir/src/lib-python/README.md" -Force

Set-Location $originalLocation

