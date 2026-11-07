# Local dev loop: apply the bundle to the Nightly worktree in a fresh folder,
# sync changed files into the kept stage, build `fast` in its own target dir.
# Nothing here touches the launcher's folders or the installed mods.
$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent
$proj = Split-Path $root -Parent
$tc = "$env:APPDATA\horizon-launcher\rust\1.99.1-x86_64-pc-windows-gnu"
$env:PATH = "$tc\bin;$tc\mingw64\bin;$env:PATH"
$env:CARGO_HOME = "$env:APPDATA\horizon-launcher\cargo"
$env:CARGO_TARGET_DIR = "$proj\horizon-target-armorhud"
$worktree = "$proj\horizon-6560641"
$rev = (git -C $worktree rev-parse HEAD).Trim()
$mods = "$proj\horizon-armorhud-mods"
$fresh = "$proj\horizon-armorhud-stage-fresh"
$kept = "$proj\horizon-armorhud-stage1"
Remove-Item -Recurse -Force $mods, $fresh -ErrorAction SilentlyContinue
New-Item -ItemType Directory $mods | Out-Null
Copy-Item -Recurse "$root\mixins\ukus_armor_hud" "$mods\"
Remove-Item -Force "$mods\ukus_armor_hud\sound\Cargo.lock" -ErrorAction SilentlyContinue
$t = "$mods\ukus_armor_hud\horizon-mixin.toml"
(Get-Content $t) -replace '^horizon_revision = ".*"', "horizon_revision = `"$rev`"" | Set-Content -Encoding ascii $t
$hm = "$env:APPDATA\horizon-launcher\native\tools\5a64af938ec8\bin\horizon-mixins.exe"
& $hm apply --workspace $worktree --mods $mods --out $fresh | Select-Object -Last 3
node "$root\tools\sync_stage.mjs" $fresh $kept
Set-Location $kept
$ErrorActionPreference = "Continue"
cmd /c "cargo build --locked --offline -p horizon-client --profile fast 2>&1" | Where-Object { $_ -match '^(error|warning: unused)|-->|^\s+\||Finished' } | Select-Object -First 60
"EXIT $LASTEXITCODE"
