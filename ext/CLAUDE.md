# Comment style

Verbose comments are fine *while* writing code — they are useful scaffolding for working out what
the code should do. Trim them as the last step before a PR merges, against the rules below.

- Module docs (`//!`) are one line. Explanation belongs on the item it describes, not at the top of
  the file.
- Don't restate the code, what the caller does with a return value, or preconditions the caller has
  already checked. Say the thing the code cannot say for itself.
- Refer to constants by name, never by value. Every hardcoded number found in a comment during the
  fp-cuda review had gone stale.
- One fact, one home: put it at the most local site and link from elsewhere if needed. Prefer the
  place where the unusual thing actually happens (e.g. the `unsafe` block, the manifest entry).
- Experiment logs — what was tried, what it gained, why an alternative was rejected — go in a
  dedicated document (see `crates/fp-cuda/EXPERIMENTS.md`) or the commit message. Never in comments,
  and never in a README, which describes the crate as it is rather than how it got there.
- Document every function, including one-line wrappers, for consistency. Every `unsafe` block gets a
  `SAFETY:` comment naming the obligations it discharges.

Rust doc comments: one-line summary, blank line, body; wrap at 100 columns. Run `cargo fmt` after
editing any Rust.
