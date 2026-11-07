#!/usr/bin/env bash
# Restarts the Horizon test client with a given ukus_armor_hud config and
# joins the test server. Usage: tools/horizon_variant.sh key=value ...
# (keys as the Java config's fields: orientation=vertical, warningBobIntensity=0 ...)
set -e
ROOT=$(cd "$(dirname "$0")/.." && pwd)
P=$(dirname "$ROOT")
HHOME=$P/horizon-armor-hud-hhome
CTL="$APPDATA/horizon-launcher/versions/nightly-2026-11-05-66f3981/horizon-ctl.exe"
RES="$APPDATA/horizon-launcher/versions/nightly-2026-11-05-66f3981/.local/horizon-client/26.3"
EXE="$P/horizon-target-armorhud/fast/horizon-client.exe"
S="--socket $HHOME/ctl.sock"
"$CTL" $S quit >/dev/null 2>&1 || true
sleep 2
DATA=$HHOME/horizon-mods/.horizon-mod-data/ukus_armor_hud
rm -rf "$DATA"; mkdir -p "$DATA"
for kv in "$@"; do printf '%s' "${kv#*=}" > "$DATA/${kv%%=*}"; done
# The config screen's key, for the screenshots.
printf 'key.ukus_armor_hud.config:key.keyboard.k\n' > "$HHOME/horizon-mods/horizon-mods.txt"
cp "$HHOME/horizon-mods/horizon-mods.txt" "$HHOME/horizon-mods.txt"
cd "$HHOME"
export HORIZON_HOME="$HHOME" HORIZON_NO_AUDIO=1 HORIZON_LAUNCHER_NO_SELF_UPDATE=1
"$CTL" $S --timeout 300 launch --game "$EXE" --resources "$RES" --width 683 --height 384 >/dev/null
"$CTL" $S --timeout 120 connect 127.0.0.1:25599 ArmorHorizon >/dev/null
"$CTL" $S --timeout 120 wait-ready >/dev/null
echo "horizon ready: $*"
