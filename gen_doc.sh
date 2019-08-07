#!/bin/sh

RUSTDOCFLAGS="--html-in-header katex-header.html" cargo doc --no-deps --document-private-items $1
