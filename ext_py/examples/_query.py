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


def query_module_only(prompt="Module", alg=None):
    """Mirror of ``ext::utils::query_module_only``.

    Query a module name (default ``S_2``) and an optional save directory, then
    construct and return a ``Resolution``. The algebra is normally selected via
    an ``@adem``/``@milnor`` suffix on the spec string, which ``construct``
    parses; ``algebra`` forces it explicitly when given.
    """
    import ext

    spec = with_default(prompt, "S_2", str)
    save_dir = optional(f"{prompt} save directory", str)
    if alg is not None:
        return ext.construct(spec, save_dir, alg)
    return ext.construct(spec, save_dir)


def query_module(alg=None):
    """Mirror of ``ext::utils::query_module``.

    Query a module, then ``Max n`` (default 30) and ``Max s`` (default 7), and
    resolve through that stem. Honors the ``SECONDARY_JOB`` environment hook the
    Rust helper uses to cap ``max_s``.
    """
    import os

    import ext

    resolution = query_module_only("Module", alg)
    max_n = with_default("Max n", "30", int)
    max_s = with_default("Max s", "7", int)

    secondary_job = os.environ.get("SECONDARY_JOB")
    if secondary_job is not None:
        s = int(secondary_job)
        if s > max_s:
            raise ValueError("SECONDARY_JOB is larger than max_s")
        max_s = min(s + 1, max_s)

    resolution.compute_through_stem(ext.sseq.Bidegree.n_s(max_n, max_s))
    return resolution
