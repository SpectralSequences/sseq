#!/usr/bin/env python3

# Build the rustdoc output published to gh-pages.
#
# `--all-features` would enable `gpu`, which needs nvcc, so every other feature is named
# explicitly. Both the feature list and the crate list are read from `cargo metadata`, so adding
# a feature or a crate needs no change here.

import json
import os
import subprocess
import sys

KATEX_HEADER = "gh-pages/katex-header.html"
EXCLUDED_FEATURES = {"default", "gpu"}


def metadata(*args):
    out = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", *args],
        stdout=subprocess.PIPE,
        check=True,
        text=True,
    )
    return json.loads(out.stdout)


def main():
    meta = metadata("--no-deps")
    packages = meta["packages"]

    features = sorted(
        f"{p['name']}/{feature}"
        for p in packages
        for feature in p["features"]
        if feature not in EXCLUDED_FEATURES
    )
    if not features:
        sys.exit("no features found; the metadata query is wrong")

    env = dict(os.environ)
    env["RUSTDOCFLAGS"] = (
        f"--html-in-header {KATEX_HEADER} {env.get('RUSTDOCFLAGS', '')}".strip()
    )

    # Not `target/`: it moves when CARGO_TARGET_DIR is set.
    doc_dir = os.path.join(metadata()["target_directory"], "doc")

    # The whole directory, not just a stale crates.js: CI caches it between runs and uploads it
    # wholesale, so docs for a crate that is no longer documented would be published forever.
    subprocess.run(["cargo", "clean", "--doc"], check=True)

    feature_args = ["--features", ",".join(features)]
    for argv in (
        ["cargo", "rustdoc", "--examples", *feature_args],
        [
            "cargo",
            "doc",
            "--all",
            "--no-deps",
            "--document-private-items",
            *feature_args,
        ],
    ):
        subprocess.run(argv, check=True, env=env)

    # Prevent the examples from showing up in the sidebar. Listing the same crates the docs were
    # built from leaves no dangling entry for one that is excluded from the workspace.
    crates = sorted(p["name"].replace("-", "_") for p in packages if p["name"] != "ext")
    entries = ",".join(f"'{name}'" for name in [*crates, "ext"])
    with open(os.path.join(doc_dir, "crates.js"), "w") as f:
        f.write(f"window.ALL_CRATES = [{entries}];\n")


if __name__ == "__main__":
    main()
