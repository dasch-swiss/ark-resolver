"""Hermetic drift check for the hand-copied interpreter path.

Named outside tests/BUILD.bazel's `test_*.py` glob (that glob wires every
match into a pytest target with fixed data) and registered as its own
py_test instead, so it can declare its own `data` deps without disturbing
every other test target's fixture set.
"""

import re
import sys

ENTRYPOINT_RE = re.compile(r'entrypoint\s*=\s*\[\s*"([^"]+)"')
PATH_RE = re.compile(r"/app/ark_resolver_bin\.runfiles/[^\"\s]*/bin/python3")


def canonical_path(build_bazel_text: str) -> str:
    match = ENTRYPOINT_RE.search(build_bazel_text)
    if match is None:
        raise AssertionError("BUILD.bazel: no oci_image entrypoint found")
    return match.group(1)


def check_file(path: str, canonical: str) -> list[str]:
    with open(path, encoding="utf-8") as f:
        text = f.read()
    occurrences = PATH_RE.findall(text)
    if not occurrences:
        return [f"{path}: no interpreter path found"]
    return [
        f"{path}: interpreter path {occurrence!r} does not match BUILD.bazel's entrypoint {canonical!r}"
        for occurrence in occurrences
        if occurrence != canonical
    ]


def main() -> int:
    with open("BUILD.bazel", encoding="utf-8") as f:
        canonical = canonical_path(f.read())

    errors: list[str] = []
    for path in ("justfile",):
        errors.extend(check_file(path, canonical))

    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
