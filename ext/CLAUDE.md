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
- Describe the arrangement as it stands, not as a change from what it replaced. A reader of the
  tree has no baseline, so "there is no X any more", "as it did before" and "still works" say
  nothing to them. Drop the contrast and state the thing itself; the before/after belongs in the
  commit message.
- Don't repeat in prose what a declaration already states — a type's size and alignment, a
  `#[repr]`, a default. Say why the declaration looks that way, if that isn't obvious.
- For a tuning knob, give the direction it pushes things in and what it trades against, then point
  at the experiment log for the numbers. Name what bounds it rather than quoting the bound, which
  goes stale like any other value.
- Something a reviewer will ask about but that no one needs a year from now — why this approach
  over the obvious one, what a surprising diff hunk is doing, what was measured — goes in the PR
  itself, as the description or a review comment on the line. It reaches the people reading the
  change and does not outlive it.
- Document every function, including one-line wrappers, for consistency. Every `unsafe` block gets a
  `SAFETY:` comment naming the obligations it discharges.

`.claude/hooks/pre-pr-comment-trim.py` blocks `git push` and `gh pr create` until that pass has
run, once per commit, so updating an open PR is checked as opening one is. `TRIM_OK=1` in front of
the command skips it for a push no one will read.

Rust doc comments: one-line summary, blank line, body; wrap at 100 columns. Run `cargo fmt` after
editing any Rust.
