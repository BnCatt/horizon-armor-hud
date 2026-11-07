#!/usr/bin/env bash
# Side-by-side screenshots: the original mod (Fabric dev client "ArmorJava",
# test harness) and the Horizon port (horizon-ctl "ArmorHorizon"), both on the
# local test server, same scene, same window (854x480) and GUI scale 2.
# Usage: tools/compare.sh NAME
#   JAVA_CMDS   extra harness lines before the shot (e.g. "set orientation VERTICAL")
#   PRE_SHOT    seconds to wait before the shots (default 2)
set -e
NAME=$1
ROOT=$(cd "$(dirname "$0")/.." && pwd)
P=$(dirname "$ROOT")
SRV=$P/horizon-armor-hud-testserver
HHOME=$P/horizon-armor-hud-hhome
HARN=$ROOT/ref/armor-hud/run/harness
CTL="$APPDATA/horizon-launcher/versions/nightly-2026-11-05-66f3981/horizon-ctl.exe"
OUT=$ROOT/screenshots/compare
mkdir -p "$OUT" "$HARN" "$HHOME/shots"

(cd "$SRV" && bash scene.sh ArmorJava >/dev/null 2>&1 && bash scene.sh ArmorHorizon >/dev/null 2>&1)
# EXTRA_RCON: more server commands, "|" between them ({P} = each player).
if [ -n "${EXTRA_RCON:-}" ]; then
  for P in ArmorJava ArmorHorizon; do
    IFS='|' read -ra CMDS <<< "${EXTRA_RCON//\{P\}/$P}"
    RCON_PORT=25598 RCON_PASS=armorhud-test py -3 -I "$HOME/.claude/skills/horizon-native-modding/scripts/rcon.py" "${CMDS[@]}" >/dev/null
  done
fi
sleep "${PRE_SHOT:-2}"
rm -f "$HARN/done.txt"
DEFAULTS='set enabled true\nset anchor HOTBAR\nset side LEFT\nset offsetX 0\nset offsetY 0\nset style HOTBAR\nset orientation HORIZONTAL\nset widgetShown NOT_EMPTY\nset offhandSlotBehavior ADHERE\nset durabilityDisplay BAR\nset offHandDurability true\nset mainHandDurability false\nset pushBossbars true\nset pushStatusEffectIcons true\nset pushSubtitles true\nset reversed false\nset iconsShown true\nset warningShown true\nset minDurabilityValue 20\nset minDurabilityPercentage 0.1\nset warningBobIntensity 3'
printf 'prep\n%b\n%b\nshot %s\n' "$DEFAULTS" "${JAVA_CMDS:-}" "java_$NAME" > "$HARN/cmd.txt"
"$CTL" --socket "$HHOME/ctl.sock" screenshot "$OUT/horizon_$NAME.png" >/dev/null
until [ -f "$HARN/done.txt" ]; do sleep 0.3; done
sleep 1.5
cp "$ROOT/ref/armor-hud/run/screenshots/java_$NAME.png" "$OUT/java_$NAME.png"
echo "$OUT/java_$NAME.png $OUT/horizon_$NAME.png"
