#!/usr/bin/env python3
"""Assertions run inside the built OCI image via `docker run -i ... - < this file`.

Piped over stdin because the distroless image has no shell to invoke a
script file directly; the entrypoint interpreter reads its program from `-c`
equivalent stdin instead. Kept as a single process: the emulated
(linux/amd64 under macOS) container is slow to start, and one process lets
every assertion share the interpreter start-up cost and report together.
"""

import glob
import importlib.util
import os
import sys

failures = []


def check(name, condition):
    status = "PASS" if condition else "FAIL"
    print(f"{status}: {name}")
    if not condition:
        failures.append(name)


# The entrypoint's own interpreter path; running this file at all already
# proves it, but assert the literal path too so a future entrypoint change
# that silently swaps interpreters is caught here rather than downstream.
interpreter = "/app/ark_resolver_bin.runfiles/rules_python++python+python_3_12_x86_64-unknown-linux-gnu/bin/python3"
check("entrypoint interpreter exists", os.path.exists(sys.executable))
check("entrypoint interpreter path matches oci_image", os.path.realpath(sys.executable) == os.path.realpath(interpreter))

# The `py_binary` stub only assembles this sys.path when actually launched via
# `ark_resolver_bin`; running the bare interpreter needs it set up by hand.
sys.path[:0] = [
    "/app/ark_resolver_bin.runfiles/_main",
    *glob.glob("/app/ark_resolver_bin.runfiles/*/site-packages"),
]

try:
    import ark_resolver._rust  # noqa: F401
    import httptools  # noqa: F401
    import sanic  # noqa: F401
    import uvloop  # noqa: F401

    check("import ark_resolver._rust, sanic, uvloop, httptools", True)
except ImportError as e:
    check(f"import ark_resolver._rust, sanic, uvloop, httptools ({e})", False)

# The interpreter's bundled pip/wheel/ensurepip must stay out of the image:
# nothing in the image needs pip, and a bundled wheel below 0.46.2 is
# vulnerable to CVE-2026-24049.
check("pip is not importable", importlib.util.find_spec("pip") is None)
check("wheel is not importable", importlib.util.find_spec("wheel") is None)
check("ensurepip is not importable", importlib.util.find_spec("ensurepip") is None)

check("ca-certificates.crt exists", os.path.exists("/etc/ssl/certs/ca-certificates.crt"))
check("zoneinfo exists", os.path.isdir("/usr/share/zoneinfo"))

# The image's own `user = "65532:65532"` (distroless nonroot's UID/GID; no
# /etc/passwd entry to name it) must apply without `--user` on this command.
DISTROLESS_NONROOT_UID = 65532
check("runs as uid 65532", os.getuid() == DISTROLESS_NONROOT_UID)

if failures:
    print(f"\n{len(failures)} check(s) failed: {', '.join(failures)}")
    sys.exit(1)

print("\nall checks passed")
sys.exit(0)
