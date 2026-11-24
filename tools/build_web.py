"""Build the web version of Hexfold into dist/.

Compiles the release wasm, runs wasm-bindgen to generate dist/hexfold.js and dist/hexfold_bg.wasm,
shrinks the wasm with wasm-opt, and replaces dist/assets/ with a fresh copy of assets/.
dist/index.html is kept as it is.
Run:  python tools/build_web.py        (then serve it:  python -m http.server -d dist)
      python tools/build_web.py --no-opt   skips wasm-opt (much faster, much bigger wasm)
Needs:  rustup target add wasm32-unknown-unknown
        cargo install wasm-bindgen-cli --version <the wasm-bindgen version in Cargo.lock>
        cargo install wasm-opt
"""
import os, re, shutil, subprocess, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DIST = os.path.join(ROOT, "dist")
TARGET = "wasm32-unknown-unknown"
WASM = os.path.join(ROOT, "target", TARGET, "release", "hexfold.wasm")


def run(*cmd):
    print(">", " ".join(cmd), flush=True)
    if subprocess.run(cmd, cwd=ROOT).returncode != 0:
        sys.exit(f"build_web: `{cmd[0]}` failed")


def check_wasm_bindgen():
    """wasm-bindgen refuses to process a module built against a different crate version."""
    lock = open(os.path.join(ROOT, "Cargo.lock"), encoding="utf-8").read()
    m = re.search(r'name = "wasm-bindgen"\r?\nversion = "([^"]+)"', lock)
    want = m.group(1) if m else None
    try:
        out = subprocess.run(["wasm-bindgen", "--version"], capture_output=True, text=True).stdout
    except FileNotFoundError:
        out = ""
    have = out.split()[-1] if out.split() else None
    if want and have != want:
        found = f"found {have}" if have else "not installed"
        sys.exit(f"build_web: wasm-bindgen {want} needed ({found}). "
                 f"Run: cargo install wasm-bindgen-cli --version {want}")


def wasm_opt(path):
    """Optimise for size in place. The --enable flags match the wasm features rustc turns on by
    default; without them wasm-opt rejects the module (or would lower the features away)."""
    if shutil.which("wasm-opt") is None:
        sys.exit("build_web: wasm-opt not installed. Run: cargo install wasm-opt  "
                 "(or pass --no-opt to skip this step)")
    before = os.path.getsize(path)
    run("wasm-opt", "-Oz", "--strip-debug",
        "--enable-bulk-memory", "--enable-nontrapping-float-to-int", "--enable-sign-ext",
        "--enable-mutable-globals", "--enable-reference-types", "--enable-multivalue",
        "-o", path, path)
    after = os.path.getsize(path)
    print(f"build_web: wasm-opt {before / 1e6:.1f} MB -> {after / 1e6:.1f} MB")


def main():
    optimise = "--no-opt" not in sys.argv[1:]
    check_wasm_bindgen()
    run("cargo", "build", "--release", "--target", TARGET)
    os.makedirs(DIST, exist_ok=True)
    run("wasm-bindgen", "--target", "web", "--no-typescript", "--out-dir", DIST,
        "--out-name", "hexfold", WASM)
    if optimise:
        wasm_opt(os.path.join(DIST, "hexfold_bg.wasm"))
    assets = os.path.join(DIST, "assets")
    shutil.rmtree(assets, ignore_errors=True)  # drop files since removed from assets/
    shutil.copytree(os.path.join(ROOT, "assets"), assets)
    if not os.path.isfile(os.path.join(DIST, "index.html")):
        print("build_web: warning: dist/index.html is missing; the build has no page to load it")
    print(f"build_web: done. Serve with: python -m http.server -d dist")


if __name__ == "__main__":
    main()
