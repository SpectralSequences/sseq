"""Python mirror of the Rust ``query`` crate (``ext/crates/query/src/lib.rs``).

Each query reads the next command-line argument and parses it; if no arguments
remain, it prompts the user on stderr and reads a line from stdin, re-prompting
until the input parses. Prompts, parse errors and logging all go to **stderr**,
so stdout stays byte-identical to the recorded benchmark output.

The example scripts drive all of their interactive input through this module so
that a single argument stream (``sys.argv[1:]``) feeds every prompt in order,
exactly as the Rust examples consume ``std::env::args()``.
"""

import sys

# The argument stream, mirroring the Rust ``ARGV`` thread-local: argv minus the
# program name, consumed left-to-right by successive queries.
_args = iter(sys.argv[1:])


def _reset_args(args=None):
    """Reset the module-level argument stream.

    Test hook (and a convenience for embedders): rebuild ``_args`` from ``args``
    (default: the current ``sys.argv[1:]``). Lets a test feed a deterministic
    answer sequence without reloading the module. Returns the new iterator.
    """
    global _args
    _args = iter(sys.argv[1:] if args is None else args)
    return _args


def raw(prompt, parser):
    """Read and parse one answer.

    If a command-line argument remains, parse it, exiting the process on failure
    (matching the Rust crate, which treats a bad CLI argument as fatal).
    Otherwise prompt on stderr and re-read from stdin until the input parses.
    """
    arg = next(_args, None)
    if arg is not None:
        print(f"{prompt}: {arg}", file=sys.stderr)
        try:
            return parser(arg)
        except Exception as e:  # noqa: BLE001 - mirror Rust's fatal CLI parse error
            print(f"{e}", file=sys.stderr)
            sys.exit(1)

    while True:
        print(f"{prompt}: ", end="", file=sys.stderr, flush=True)
        line = sys.stdin.readline()
        # At EOF, read_line yields "": treated as an empty answer, exactly as the
        # Rust crate does. Empty answers are valid for optional/with_default/yes_no
        # (they fall back to the default); a parser that rejects empty input would
        # loop forever on repeated EOF in Rust, so we exit instead of hanging.
        at_eof = line == ""
        try:
            return parser(line.strip())
        except Exception as e:  # noqa: BLE001 - mirror Rust's retry loop
            if at_eof:
                print(f"{e}", file=sys.stderr)
                sys.exit(1)
            print(f"{e}\n\nTry again", file=sys.stderr)


def with_default(prompt, default, parser):
    """Query with a default used when the answer is empty."""

    def parse(x):
        return parser(default) if x == "" else parser(x)

    return raw(f"{prompt} (default: {default})", parse)


def optional(prompt, parser):
    """Query an optional value; an empty answer yields ``None``."""

    def parse(x):
        return None if x == "" else parser(x)

    return raw(f"{prompt} (optional)", parse)


def yes_no(prompt):
    """Query a yes/no answer, defaulting to yes."""

    def parse(response):
        if response.startswith("y") or response.startswith("n"):
            return response.startswith("y")
        raise ValueError(
            f"unrecognized response '{response}'. Should be '(y)es' or '(n)o'"
        )

    return with_default(prompt, "y", parse)


def vector(prompt, length):
    """Query a vector written as ``[a, b, c]`` with a fixed length."""

    def parse(s):
        v = [int(x.strip()) for x in s[1 : len(s) - 1].split(",")]
        if len(v) != length:
            raise ValueError(
                f"Target has dimension {length} but {len(v)} coordinates supplied"
            )
        return v

    return raw(prompt, parse)
