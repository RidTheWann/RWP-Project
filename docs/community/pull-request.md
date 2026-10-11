# Pull request template

## Batch scope

One batch, one purpose. List the files changed and why each is included.

## Verification (paste, don't claim)

- Gate commands per `docs/local-gates.md` with pass/fail plus numbers:
- New tests added (behavior changes require regression tests first):
- Docs updated (CHANGELOG for user-facing changes, STATE evidence line):

## Invariants

- [ ] No threshold, detector, or schema-semantic change without Tier justification.
- [ ] No version bump outside the release process.
- [ ] No secrets, private paths, or unredacted fixtures in the diff.
- [ ] No hardware claims beyond recorded evidence.
