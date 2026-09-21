#!/usr/bin/env python3
"""Block the first push or PR command on a branch until the comment-trim pass has run."""

import json
import os
import re
import subprocess
import sys
from typing import cast

# The three ways work reaches a reviewer. Each has to sit where a command starts -- at the front of
# the string or after a separator -- so that quoting one inside a heredoc or a commit message is
# not mistaken for running it.
START = r"(?:^|[;&|\n]|&&|\|\|)\s*"
TRIGGER = re.compile(START + r"(?:\w+=\S+\s+)*(?:git\s+push|gh\s+pr\s+(?:create|edit))\b")

# An explicit opt-out, for a push that is not review-bound (a backup branch, a CI retrigger).
OPT_OUT = re.compile(r"\bTRIM_OK=1\b")

REMINDER = """Comment-trim pass required before this reaches a reviewer (branch: {branch}).

Read ext/CLAUDE.md in full, then go through the comments this branch adds or touches:

    git diff master...HEAD

Check each one against every rule there, in particular:
  - module docs are one line
  - no restating the code, and no before/after framing
  - constants by name, never by value
  - one fact, one home
  - experiment logs and measurements belong in EXPERIMENTS.md or the commit message
  - reviewer-only context belongs in the PR body or an inline review comment
  - every function documented, every `unsafe` block carrying SAFETY:
  - doc comments wrap at 100 columns; run `cargo fmt`

Apply the trims, commit them, then re-run the command. This fires once per commit, so a trim
commit is itself checked before it goes out; a re-run with nothing new to say passes straight
through. Prefix the command with TRIM_OK=1 to skip it for a push that is not review-bound."""


def field(obj: object, key: str) -> object:
    """Return `obj[key]`, or None unless `obj` is a mapping holding `key`."""
    return cast("dict[str, object]", obj).get(key) if isinstance(obj, dict) else None


def git(*args: str) -> str:
    """Run a git command in the cwd and return its stdout, stripped."""
    return subprocess.run(
        ["git", *args], capture_output=True, text=True, check=True
    ).stdout.strip()


def main() -> int:
    """Decide whether to let the Bash command through, per the PreToolUse hook protocol.

    CLAUDE.md asks for the trim pass as the last step before a PR merges, which is exactly the
    step an agent heading for a PR skips. Anything unexpected on stdin lets the command through: a
    hook that cannot read its own input has no business stopping work.
    """
    try:
        payload = cast(object, json.load(sys.stdin))
    except (json.JSONDecodeError, UnicodeDecodeError):
        return 0
    command = field(field(payload, "tool_input"), "command")
    if not isinstance(command, str):
        return 0

    if not TRIGGER.search(command) or OPT_OUT.search(command):
        return 0

    try:
        git_dir = git("rev-parse", "--absolute-git-dir")
        branch = git("rev-parse", "--abbrev-ref", "HEAD")
        head = git("rev-parse", "HEAD")
    except (subprocess.CalledProcessError, FileNotFoundError):
        return 0

    # The marker lives in .git, so it is per-clone, never committed, and goes with the worktree. It
    # records the commit it cleared, so a branch that has gained commits since is checked again --
    # updating an open PR puts new commits in front of a reviewer just as opening one did.
    marker = os.path.join(git_dir, "claude-comment-trim", branch.replace("/", "%2F"))
    try:
        with open(marker) as f:
            if f.read().strip() == head:
                return 0
    except FileNotFoundError:
        pass

    os.makedirs(os.path.dirname(marker), exist_ok=True)
    with open(marker, "w") as f:
        _ = f.write(head + "\n")

    # Exit 2 is the contract for "refuse the call and hand this text back to the model".
    print(REMINDER.format(branch=branch), file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main())
