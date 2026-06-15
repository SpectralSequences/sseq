#!/usr/bin/env python3
"""Serve the sseq_ext web playground.

This is a tiny static file server rooted at the `python_ext/` directory so that
both the playground page (`playground/index.html`) and the Pyodide wheel
(`dist/*wasm32.whl`) are reachable over HTTP. It also exposes a small
`/playground/wheel.json` endpoint that reports the current wheel's URL, so the
page does not need the (version-dependent) wheel filename hard-coded.

Usage:
    python playground/serve.py [port]      # default port 8000

Then open http://localhost:8000/playground/ in a browser.

Note: you must build the Pyodide wheel first (see ../README.md):
    ./build-pyodide.sh
"""

from __future__ import annotations

import glob
import http.server
import json
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))  # python_ext/
DIST = os.path.join(ROOT, "dist")
REPO = os.path.dirname(ROOT)  # sseq/
MODULES = os.path.join(REPO, "ext", "steenrod_modules")


def find_wheel() -> str | None:
    matches = sorted(glob.glob(os.path.join(DIST, "*wasm32.whl")))
    if not matches:
        return None
    # Prefer the most recently modified wheel.
    matches.sort(key=os.path.getmtime, reverse=True)
    return os.path.basename(matches[0])


class Handler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=ROOT, **kwargs)

    def do_GET(self):  # noqa: N802
        if self.path.rstrip("/") == "/playground/wheel.json":
            return self.serve_wheel_json()
        if self.path.rstrip("/") == "/playground/modules.json":
            return self.serve_modules_json()
        return super().do_GET()

    def serve_modules_json(self):
        """Return {name: raw-json-text} for every steenrod module definition.

        The page writes these into Pyodide's virtual filesystem under
        `steenrod_modules/` so `ext.construct("S_2")` can find them.
        """
        modules = {}
        for path in sorted(glob.glob(os.path.join(MODULES, "*.json"))):
            name = os.path.splitext(os.path.basename(path))[0]
            with open(path, "r", encoding="utf-8") as f:
                modules[name] = f.read()
        self._send_json(modules)

    def serve_wheel_json(self):
        wheel = find_wheel()
        if wheel is None:
            self.send_error(
                404,
                "No wasm32 wheel found in dist/. Build it with ./build-pyodide.sh",
            )
            return
        self._send_json({"url": f"/dist/{wheel}"})

    def _send_json(self, obj):
        body = json.dumps(obj).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        # Disable caching so freshly built/edited files are always picked up.
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(body)

    def end_headers(self):
        # Allow correct MIME for .whl downloads via fetch.
        super().end_headers()

    def log_message(self, fmt, *args):  # quieter logging
        sys.stderr.write("%s - %s\n" % (self.address_string(), fmt % args))


def main() -> int:
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8000
    wheel = find_wheel()
    if wheel is None:
        print(
            "WARNING: no *wasm32.whl found in dist/.\n"
            "         Build it first:  ./build-pyodide.sh\n",
            file=sys.stderr,
        )
    else:
        print(f"Serving wheel: dist/{wheel}")

    server = http.server.ThreadingHTTPServer(("127.0.0.1", port), Handler)
    url = f"http://localhost:{port}/playground/"
    print(f"Playground: {url}")
    print("Press Ctrl+C to stop.")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nbye")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
