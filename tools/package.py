#!/usr/bin/env python3
"""Packages mixins/ukus_armor_hud as target/ukus_armor_hud-<version>.hnmod.

Source only, deterministic (fixed dates and modes, sorted entries): the launcher
compiles it. Adds LICENSE (MIT, the original's notice) and README.md at the root.
Does NOT install anything.  Usage: py -3 tools/package.py
"""
import hashlib
import json
import sys
import tomllib
import zipfile
from pathlib import Path

root = Path(__file__).resolve().parents[1]
bundle = root / "mixins" / "ukus_armor_hud"
compat = json.loads((root / "compatibility.json").read_text(encoding="utf-8"))
mixin = tomllib.loads((bundle / "horizon-mixin.toml").read_text(encoding="utf-8"))
mod = mixin["mod"]
if mixin["compatibility"]["horizon_revision"] != compat["horizon_revision"]:
    sys.exit("horizon-mixin.toml and compatibility.json pin different Horizon revisions")
for lock in bundle.rglob("Cargo.lock"):
    sys.exit(f"remove {lock}: a mod crate builds on Horizon's Cargo.lock")

manifest = dict(schema=1, id=mod["id"], version=mod["version"], game=compat["game"],
                horizon=compat["horizon_revision"], mixins=compat["horizon_mixins_revision"],
                hooks=compat["hooks"])
files = {"horizon-native.json": (json.dumps(manifest, indent=2, sort_keys=True) + "\n").encode()}
# The launcher refuses anything but horizon-native.json and mixins/** ("unexpected native source");
# the MIT notice is in core.rs.
for name in ():
    files[name] = (root / name).read_bytes().replace(b"\r\n", b"\n")
for path in sorted(bundle.rglob("*")):
    if path.is_file() and path.suffix in (".rs", ".toml"):
        files[f"mixins/{bundle.name}/{path.relative_to(bundle).as_posix()}"] = path.read_bytes().replace(b"\r\n", b"\n")

out = root / "target" / f"{mod['id']}-{mod['version']}.hnmod"
out.parent.mkdir(exist_ok=True)
with zipfile.ZipFile(out, "w", compression=zipfile.ZIP_DEFLATED) as z:
    for name, content in sorted(files.items()):
        info = zipfile.ZipInfo(name, (2026, 1, 1, 0, 0, 0))
        info.compress_type = zipfile.ZIP_DEFLATED
        info.external_attr = 0o100644 << 16
        z.writestr(info, content)
print(json.dumps(dict(file=str(out), files=len(files), size=out.stat().st_size,
                      sha256=hashlib.sha256(out.read_bytes()).hexdigest()), indent=2))
