"""Bazel `py_test` entry point that runs pytest against a single test module.

Runfiles are read-only, and `bazel test` sets the working directory to the
runfiles root of this repo (`_main`), so `pyproject.toml` and
`tests/ark-registry.ini` resolve at their normal repo-relative paths without
needing the runfiles-lookup library.
"""

import os
import sys

import pytest


def main() -> None:
    args = [
        # -c also applies pyproject.toml's [tool.pytest.ini_options] addopts
        # ("--verbose -s"), so those flags are not repeated here.
        "-c",
        os.path.abspath("pyproject.toml"),
        "-p",
        "no:cacheprovider",
        f"--basetemp={os.environ['TEST_TMPDIR']}",
        f"--junitxml={os.environ['XML_OUTPUT_FILE']}",
        *sys.argv[1:],
    ]
    sys.exit(pytest.main(args))


if __name__ == "__main__":
    main()
