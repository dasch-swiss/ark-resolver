"""
Comparative parity tests for redirect URL generation.

Runs the same inputs through Python and Rust ArkUrlInfo.to_redirect_url()
and asserts identical output. Covers all redirect scenarios: top-level,
project, resource, value, v0 salsah, case variants, and timestamps.
"""

import os

import pytest

from ark_resolver import ark
from ark_resolver._rust import load_settings as load_settings_rust  # type: ignore[import-untyped]
from ark_resolver.ark_url import ArkUrlInfo as PythonArkUrlInfo
from ark_resolver.ark_url_rust import ArkUrlInfo as RustArkUrlInfo


@pytest.fixture(scope="module")
def python_settings():
    os.environ["ARK_REGISTRY"] = "tests/ark-registry.ini"
    return ark.load_settings()


@pytest.fixture(scope="module")
def rust_settings():
    os.environ["ARK_REGISTRY"] = "tests/ark-registry.ini"
    return load_settings_rust()


@pytest.mark.parametrize(
    "ark_id",
    [
        # Top-level object
        "ark:/00000/1",
        # Project with default host
        "ark:/00000/1/0003",
        # Project with custom host
        "ark:/00000/1/0004",
        # Project on Salsah with custom host
        "ark:/00000/1/0006",
        # v1 project - uppercase input
        "ark:/00000/1/080E",
        # v1 project - lowercase input (tests case normalization)
        "ark:/00000/1/080e",
        # Resource without timestamp
        "ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn",
        # Resource with timestamp (fractional part)
        "ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn.20180604T085622513Z",
        # Resource with timestamp (no fractional part)
        "ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn.20180604T085622Z",
        # Resource with custom redirect pattern
        "ark:/00000/1/0005/0_sWRg5jT3S0PLxakX9ffg1",
        # Value without timestamp (custom redirect pattern)
        "ark:/00000/1/0005/SQkTPdHdTzq_gqbwj6QR=AR/=SSbnPK3Q7WWxzBT1UPpRgo",
        # Value with timestamp
        "ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn/pLlW4ODASumZfZFbJdpw1gu.20180604T085622Z",
        # v0 salsah without timestamp
        "ark:/00000/0002-779b9990a0c3f-6e",
        # v0 salsah with timestamp
        "ark:/00000/0002-779b9990a0c3f-6e.20190129",
        # v0 salsah lowercase project without timestamp
        "ark:/00000/080e-76bb2132d30d6-0",
        # v0 salsah lowercase project with timestamp
        "ark:/00000/080e-76bb2132d30d6-0.20190129",
        # v0 salsah with too-short timestamp (treated as no timestamp)
        "ark:/00000/080e-76bb2132d30d6-0.2019111",
    ],
    ids=[
        "top-level",
        "project-default-host",
        "project-custom-host",
        "project-salsah-host",
        "project-uppercase",
        "project-lowercase",
        "resource",
        "resource-ts-fractional",
        "resource-ts-no-fractional",
        "resource-custom-pattern",
        "value-custom-pattern",
        "value-with-ts",
        "v0-salsah",
        "v0-salsah-ts",
        "v0-salsah-lowercase",
        "v0-salsah-lowercase-ts",
        "v0-salsah-short-ts",
    ],
)
def test_redirect_url_parity(python_settings, rust_settings, ark_id):
    """Python and Rust must produce identical redirect URLs."""
    python_url = PythonArkUrlInfo(python_settings, ark_id).to_redirect_url()
    rust_url = RustArkUrlInfo(rust_settings, ark_id).to_redirect_url()
    assert python_url == rust_url, (
        f"Parity mismatch for {ark_id}:\n"
        f"  Python: {python_url}\n"
        f"  Rust:   {rust_url}"
    )


@pytest.mark.parametrize(
    "ark_id",
    [
        "ark:/00000/1/ZZZZ",  # unknown project
        "ark:/00000/1/0001/cmfk1DMHRBir4=_6HXpEFAn",  # wrong check digit
    ],
    ids=[
        "unknown-project",
        "bad-check-digit",
    ],
)
def test_error_parity(python_settings, rust_settings, ark_id):
    """Both Python and Rust should fail for invalid ARK IDs."""
    python_error = None
    rust_error = None
    try:
        PythonArkUrlInfo(python_settings, ark_id).to_redirect_url()
    except Exception as e:
        python_error = type(e).__name__
    try:
        RustArkUrlInfo(rust_settings, ark_id).to_redirect_url()
    except Exception as e:
        rust_error = type(e).__name__
    assert (python_error is not None) == (rust_error is not None), (
        f"Error parity mismatch for {ark_id}:\n"
        f"  Python error: {python_error}\n"
        f"  Rust error:   {rust_error}"
    )
