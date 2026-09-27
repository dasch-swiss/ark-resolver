#!/usr/bin/env bash
# Bazel workspace-status script.
#
# Wired by `.bazelrc`'s `--workspace_status_command=tools/workspace_status.sh`.
# Bazel runs this on every build and parses each line as a `KEY VALUE` pair.
# Keys prefixed `STABLE_*` invalidate downstream actions only when the value
# *changes*. Editing a `.rs`/`.py` file does not restamp the binary/image
# (the values stay identical) but bumping `version.txt` or committing does.
#
# `version.txt` is required: `set -euo pipefail` plus reading it into a
# variable before echoing it means a missing/unreadable file fails this
# script, and therefore the Bazel build, loudly. Git metadata is best-effort:
# builds from a source tarball without `.git` still work, falling back to
# `unknown`/epoch as noted below.

set -euo pipefail

# `BUILD_WORKSPACE_DIRECTORY` is set by `bazel run` but not by `bazel build`;
# the workspace-status script always runs from the workspace root, so
# falling back to `.` is correct.
cd "${BUILD_WORKSPACE_DIRECTORY:-.}"

# Commit SHA: pins the binary/image to its source for debugging and release tracking.
echo "STABLE_GIT_COMMIT $(git rev-parse HEAD 2>/dev/null || echo unknown)"

# `version.txt` is the single source of truth for the released version string.
# No fallback here: `set -e` does not see failures inside `$(...)` when that
# substitution is just one argument to `echo`, so the version is read into a
# variable first, letting a missing/unreadable file fail the script.
version="$(tr -d '[:space:]' < version.txt)"
echo "STABLE_ARK_VERSION $version"

# Image-creation timestamp pinned to the commit's date, not wall clock, so
# two builds of the same source from the same lockfile produce byte-identical
# OCI image tarballs. `%cI` is the committer date.
echo "STABLE_IMAGE_CREATED $(git log -1 --format=%cI 2>/dev/null || echo 1970-01-01T00:00:00Z)"

# Image tag, defined only here: `<version>` when HEAD sits exactly on a git
# tag, else `<version>-<shortsha>`; both git calls fall back gracefully so a
# tarball build without `.git` still stamps something.
short_sha="$(git log --pretty=format:'%h' -n 1 2>/dev/null || echo unknown)"
if git describe --tags --exact-match >/dev/null 2>&1; then
    echo "STABLE_IMAGE_TAG $version"
else
    echo "STABLE_IMAGE_TAG ${version}-${short_sha}"
fi
