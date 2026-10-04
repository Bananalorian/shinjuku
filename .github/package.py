#!/usr/bin/env python3
"""Zip a desktop build for a GitHub Release. Usage: package.py <platform-name>"""
import os, sys, zipfile

name = sys.argv[1]
out = f"last-train-to-shinjuku-{name}.zip"
readme = """LAST TRAIN TO SHINJUKU
Clear five Tokyo stations up the Yamanote line, from Akihabara to Shinjuku.

Keyboard + mouse: WASD move, mouse aim + fire, Space dash, right click grenade,
Esc pause, M mute. Controllers: left stick move, right stick aim + fire, A dash,
RB grenade, Start pause.

Windows: if SmartScreen warns you, click "More info" then "Run anyway".
macOS: the first time, right-click the app and choose Open (it isn't signed).
Linux: run ./shinjuku (you may need: chmod +x shinjuku).

https://github.com/Bananalorian/shinjuku
"""

def add_exec(z, path, arcname):
    info = zipfile.ZipInfo(arcname)
    info.external_attr = (0o755 & 0xFFFF) << 16
    info.compress_type = zipfile.ZIP_DEFLATED
    with open(path, "rb") as f:
        z.writestr(info, f.read())

with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
    z.writestr("READ ME.txt", readme)
    if name.startswith("windows"):
        z.write("target/release/shinjuku.exe", "Last Train to Shinjuku.exe")
    elif name.startswith("macos"):
        app = "Last Train to Shinjuku.app/Contents"
        plist = f"""<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>Last Train to Shinjuku</string>
  <key>CFBundleDisplayName</key><string>Last Train to Shinjuku</string>
  <key>CFBundleIdentifier</key><string>io.github.bananalorian.shinjuku</string>
  <key>CFBundleExecutable</key><string>shinjuku</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>{os.environ.get("GITHUB_REF_NAME", "dev").lstrip("v")}</string>
  <key>NSHighResolutionCapable</key><true/>
</dict></plist>
"""
        z.writestr(f"{app}/Info.plist", plist)
        add_exec(z, "target/release/shinjuku", f"{app}/MacOS/shinjuku")
    else:
        add_exec(z, "target/release/shinjuku", "shinjuku")
print("wrote", out)
