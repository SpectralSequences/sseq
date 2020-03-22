#!/bin/sh

RUSTDOCFLAGS="--html-in-header gh-pages/katex-header.html" cargo doc --all --no-deps --document-private-items $1
