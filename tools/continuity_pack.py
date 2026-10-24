#!/usr/bin/env python3
"""Builds resourcepacks/Continuity Default.zip from Continuity's own bundled default pack.

usage: python tools/continuity_pack.py PATH/TO/continuity-3.0.1+26.3.jar [OUT.zip]

The jar (https://modrinth.com/mod/continuity, LGPL-3.0) carries `resourcepacks/default`: the
OptiFine `optifine/ctm` rules and the artwork for connected glass (plain, stained, tinted, panes),
bookshelves and sandstone. The programmer art variants are left out: Continuity applies them only
with that resource pack, through its `resourceCondition`.
"""
import json
import sys
import zipfile
from pathlib import Path

jar = Path(sys.argv[1])
out = Path(sys.argv[2]) if len(sys.argv) > 2 else Path('resourcepacks') / 'Continuity Default.zip'
out.parent.mkdir(parents=True, exist_ok=True)
prefix = 'resourcepacks/default/'
count = 0
with zipfile.ZipFile(jar) as src, zipfile.ZipFile(out, 'w', zipfile.ZIP_DEFLATED) as dst:
    meta = {
        'pack': {
            'pack_format': 46,
            'supported_formats': {'min_inclusive': 46, 'max_inclusive': 255},
            'min_format': 46,
            'max_format': 255,
            'description': "§bContinuity§r's default connected textures: glass, bookshelves, sandstone",
        }
    }
    dst.writestr('pack.mcmeta', json.dumps(meta))
    for name in src.namelist():
        if not name.startswith(prefix) or name.endswith('/'):
            continue
        rel = name[len(prefix):]
        if rel == 'pack.mcmeta' or '/optifine/ctm/programmer_art/' in rel:
            continue
        dst.writestr(rel, src.read(name))
        count += 1
print(f'{count} files -> {out}')
