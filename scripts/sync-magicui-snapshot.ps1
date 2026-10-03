# Refresh the Magic UI registry fixture used by the example inventory test.
#
# The fixture is derived from Magic UI (MIT) and is consumed by
# crates/enchant-iced/tests/example_inventory.rs. Nothing in this repository
# depends on a Magic UI checkout at build time — this is a maintenance script.
#
# Usage:
#   .\scripts\sync-magicui-snapshot.ps1
#   .\scripts\sync-magicui-snapshot.ps1 -SourceRoot D:\checkouts\magicui
#
# SourceRoot must point at a checkout of https://github.com/magicuidesign/magicui

param(
    [string]$SourceRoot = (Join-Path (Split-Path (Split-Path $PSScriptRoot -Parent) -Parent) "magic-refs\magicui")
)

$ErrorActionPreference = "Stop"
$Root = Split-Path $PSScriptRoot -Parent
$UpstreamFile = Join-Path $SourceRoot "apps\www\registry\registry-examples.ts"
$FixtureDest = Join-Path $Root "crates\enchant-iced\tests\fixtures\registry-examples.ts"

if (-not (Test-Path $UpstreamFile)) {
    throw "Magic UI checkout not found: $UpstreamFile`nPass -SourceRoot or clone https://github.com/magicuidesign/magicui"
}

New-Item -ItemType Directory -Force -Path (Split-Path $FixtureDest -Parent) | Out-Null
Copy-Item $UpstreamFile $FixtureDest -Force

$ref = (& git -C $SourceRoot rev-parse HEAD 2>$null)
Write-Host "synced: crates/enchant-iced/tests/fixtures/registry-examples.ts"
Write-Host "source: https://github.com/magicuidesign/magicui @ $ref"
Write-Host ""
Write-Host "Next: cargo test -p enchant-iced --test example_inventory"
Write-Host "New upstream components may need examples, and new public helpers may need"
Write-Host "an entry in HELPER_EXPORTS in that test."
