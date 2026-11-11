Fixes DEV-

<!--
This repo rebase-merges: every commit on the branch lands on main as it is, and release-please turns each one into a CHANGELOG line. Clean up the branch before merge: one logical change per commit, each a Conventional Commit (CONVENTIONS.md, Commit Conventions). `just commit-lint` checks them locally.
-->

## Summary

<!-- What changed and why, in 1-3 bullets. -->

## Served and Shadow implementation

<!-- If the change touches resolution logic: which implementation it changes, and whether the other one still agrees (CONVENTIONS.md, Served and Shadow Implementations). Delete this section otherwise. -->

## Test Plan

- [ ] `just check`, `just test` and `just pytest` pass
- [ ] `just smoke-test`, if the image, the interpreter or the Python dependencies changed

## Review guide

<!-- Where to start reading, and anything a reviewer should not miss. Delete if the diff speaks for itself. -->
