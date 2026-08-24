#!/usr/bin/env sh
# Regression suite for check-conventional-subject.py.
#
# That script gates two things at once — every branch commit (via lefthook) and every
# PR title (via the pr-title workflow, which is the string that reaches `main`). A
# silent loosening of its regex would disarm both at the same time, so the cases below
# run in CI before the title itself is checked.
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
validator="$script_dir/check-conventional-subject.py"

pass=0
fail=0

# expect <expected-exit> <subject> — prefer the valid/invalid wrappers below.
expect() {
	expected=$1
	subject=$2
	set +e
	printf '%s\n' "$subject" | python3 "$validator" >/dev/null 2>&1
	actual=$?
	set -e
	if [ "$actual" -eq "$expected" ]; then
		pass=$((pass + 1))
	else
		fail=$((fail + 1))
		printf 'FAIL  want exit %s, got %s:  %s\n' "$expected" "$actual" "$subject" >&2
	fi
}

valid() { expect 0 "$1"; }
invalid() { expect 1 "$1"; }

# One per type, so shrinking the type list trips a test.
valid 'build(deps): bump crustywad to 0.9.5'
valid 'chore: bootstrap tooling'
valid 'ci: fail the build when a Codecov upload fails'
valid 'docs(adr): record the squash-merge changelog contract'
valid 'feat(parser): add wad reader'
valid 'fix: correct off-by-one'
valid 'perf: cache sector lookups'
valid 'refactor(crustyview-core): split summary'
valid 'revert: feat(parser): add wad reader'
valid 'style: rustfmt'
valid 'test: cover the blockmap arm'

# Breaking marker, with and without a scope — the only channel for a breaking change,
# since the squash body is BLANK by policy.
valid 'feat!: drop the legacy loader'
valid 'feat(map2d)!: change the textureRgba contract'

# Scope punctuation the regex allows.
valid 'chore(web/src): move a helper'
valid 'fix(crusty-view.core_2): odd but legal scope'

# Trailing whitespace after a real description is harmless.
valid 'feat: add wad reader '

invalid 'chore/83 versioning policy' # branch name; what `gh pr create --fill` produces
invalid 'Add sector overlay'         # no type
invalid 'feat add wad reader'        # no colon
invalid 'feat:'                      # no space, no description
invalid 'feat: '                     # empty description
invalid 'feat:  '                    # whitespace-only description
invalid 'feat:	'                     # tab-only description
invalid 'Feat: add wad reader'       # capitalized type
invalid 'FEAT: add wad reader'
invalid 'feature: add wad reader'      # not in the type list
invalid 'feat(Parser): add wad reader' # capitalized scope
invalid 'feat(): add wad reader'       # empty scope
invalid '  feat: leading space'
invalid ''

printf '%s passed, %s failed\n' "$pass" "$fail"
[ "$fail" -eq 0 ]
