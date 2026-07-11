# Copy only wasm-demo inputs from magic-refs into the project.
# magic-refs is read-only study material — the project must not depend on it at build time.
# Usage: .\scripts\sync-magicui-snapshot.ps1

$ErrorActionPreference = "Stop"
$Root = Split-Path $PSScriptRoot -Parent
$SourceRoot = Join-Path (Split-Path $Root -Parent) "magic-refs\magicui\apps\www"
$DestRoot = Join-Path $Root "crates\wasm-demo\upstream\magicui-www"
$FixtureDest = Join-Path $Root "crates\magic-iced\tests\fixtures\registry-examples.ts"

if (-not (Test-Path $SourceRoot)) {
    throw "magic-refs snapshot missing: $SourceRoot`nRun magic-refs\clone-refs.ps1 and sync-stable.ps1 first."
}

function Sync-Tree($relative) {
    $from = Join-Path $SourceRoot $relative
    $to = Join-Path $DestRoot $relative
    if (-not (Test-Path $from)) {
        Write-Warning "skip missing: $relative"
        return
    }
    New-Item -ItemType Directory -Force -Path (Split-Path $to -Parent) | Out-Null
    if (Test-Path $to) {
        Remove-Item $to -Recurse -Force
    }
    robocopy $from $to /E /NFL /NDL /NJH /NJS /nc /ns /np | Out-Null
    Write-Host "synced: $relative"
}

function Sync-File($relative) {
    $from = Join-Path $SourceRoot $relative
    $to = Join-Path $DestRoot $relative
    if (-not (Test-Path $from)) {
        Write-Warning "skip missing: $relative"
        return
    }
    New-Item -ItemType Directory -Force -Path (Split-Path $to -Parent) | Out-Null
    Copy-Item $from $to -Force
    Write-Host "synced: $relative"
}

# Remove previous full-site snapshot leftovers.
if (Test-Path $DestRoot) {
    Remove-Item $DestRoot -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $DestRoot | Out-Null

Sync-Tree "config"
Sync-File "styles/globals.css"
Sync-File "app/layout.tsx"
Sync-File "components/site-banner.tsx"
Sync-File "components/command-menu.tsx"
Sync-File "components/sections/hero.tsx"
Sync-File "components/sections/showcase.tsx"
Sync-File "components/sections/testimonials.tsx"
Sync-File "components/sections/video-testimonials.tsx"
Sync-Tree "content/docs/components"
Sync-Tree "content/showcase"
Sync-Tree "public/r"

New-Item -ItemType Directory -Force -Path (Split-Path $FixtureDest -Parent) | Out-Null
Copy-Item (Join-Path $SourceRoot "registry/registry-examples.ts") $FixtureDest -Force
Write-Host "synced: crates/magic-iced/tests/fixtures/registry-examples.ts"

$ref = Push-Location (Join-Path (Split-Path $Root -Parent) "magic-refs\magicui"); git rev-parse HEAD; Pop-Location
@"
magicui upstream snapshot (wasm-demo build inputs only)
source: https://github.com/magicuidesign/magicui
ref: $ref
copied_from: magic-refs/magicui/apps/www
sync: scripts/sync-magicui-snapshot.ps1

included:
  config/
  styles/globals.css
  app/layout.tsx
  components/site-banner.tsx
  components/command-menu.tsx
  components/sections/hero.tsx
  components/sections/showcase.tsx
  components/sections/testimonials.tsx
  components/sections/video-testimonials.tsx
  content/docs/components/
  content/showcase/
  public/r/
  crates/magic-iced/tests/fixtures/registry-examples.ts
"@ | Set-Content (Join-Path $DestRoot "SOURCE.txt")

Write-Host "`nDone. Rebuild wasm-demo to regenerate reference-data.json."
