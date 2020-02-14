#!/bin/sh

RUSTDOCFLAGS="--html-in-header katex-header.html" cargo doc --all --no-deps --document-private-items $1
