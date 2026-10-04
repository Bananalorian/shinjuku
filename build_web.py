#!/usr/bin/env python3
"""Pack the WASM build + macroquad's JS loader into one self-contained index.html.

    cargo build --release --target wasm32-unknown-unknown
    python3 build_web.py            # writes dist/index.html

The single file works on itch.io (upload as an HTML game) or any static host.
"""
import base64, glob, os, sys

ROOT = os.path.dirname(os.path.abspath(__file__))
wasm = os.path.join(ROOT, "target/wasm32-unknown-unknown/release/shinjuku.wasm")
if not os.path.exists(wasm):
    sys.exit("build the wasm first: cargo build --release --target wasm32-unknown-unknown")

# use the loader that ships with the exact macroquad version we compiled against
home = os.path.expanduser("~")
bundles = sorted(glob.glob(os.path.join(home, ".cargo/registry/src/*/macroquad-0.4.*/js/mq_js_bundle.js")))
if not bundles:
    sys.exit("couldn't find macroquad's mq_js_bundle.js in ~/.cargo/registry")
bundle = open(bundles[-1], encoding="utf-8").read()

html = open(os.path.join(ROOT, "web/template.html"), encoding="utf-8").read()
b64 = base64.b64encode(open(wasm, "rb").read()).decode("ascii")
html = html.replace("/*__MQ_BUNDLE__*/", bundle).replace("__WASM_B64__", b64)
os.makedirs(os.path.join(ROOT, "dist"), exist_ok=True)
out = os.path.join(ROOT, "dist/index.html")
open(out, "w", encoding="utf-8").write(html)
print(f"wrote {out} ({len(html) / 1e6:.2f} MB)")
