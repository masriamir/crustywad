#!/usr/bin/env python3
"""Validate a Conventional Commits subject line, read from stdin.

Single source of truth for two gates that must agree but see different strings:

  * lefthook's `commit-msg` hook, which checks each commit written on a branch;
  * the `pr-title` CI workflow, which checks the PR title.

Only the second one reaches `main`. PRs squash-merge, so the PR title becomes the
sole commit message on `main` and is what the changelog/version tooling parses for
inclusion, section, and version bump. Keeping one validator means a title that
satisfies CI cannot be one lefthook would have rejected, or vice versa.

Reads stdin and considers only its first line, so a whole commit-message file can be
redirected in as readily as a bare PR title is piped in. Callers do not need to slice
off the subject themselves.
"""

import re
import sys

TYPES = (
    "build",
    "chore",
    "ci",
    "docs",
    "feat",
    "fix",
    "perf",
    "refactor",
    "revert",
    "style",
    "test",
)

# Conventional Commits as the tooling parses it: lowercase type, an optional
# parenthesized scope, an optional `!` breaking marker, then `: ` and a
# description.
#
# The description is `.*\S`, not `.+`: `.+` would accept `feat:  `, since the literal
# `: ` consumes the colon and the first space and `.+` is then satisfied by the second
# one. Requiring a non-whitespace character rejects a blank description without
# rejecting a legitimate one that happens to carry trailing whitespace.
PATTERN = re.compile(
    rf"^({'|'.join(TYPES)})(\([a-z0-9._/-]+\))?!?: .*\S",
)


def main() -> int:
    subject = sys.stdin.readline().rstrip("\n")
    if PATTERN.match(subject):
        return 0

    print(f"Not a Conventional Commits subject: {subject!r}", file=sys.stderr)
    print(
        "Expected <type>[(scope)][!]: <description>, e.g. "
        "feat(parser): add wad reader",
        file=sys.stderr,
    )
    print(f"Valid types: {' '.join(TYPES)}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
