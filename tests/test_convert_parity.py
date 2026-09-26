"""
Comparative parity tests for the convert pipeline.

Runs the same inputs through the Python and Rust implementations of each step the
/convert route chains (to_resource_iri, get_timestamp, resource_iri_to_ark_id) and
asserts identical output per step, so a mismatch names the step that diverged.
"""

import os

import pytest
from ark_resolver._rust import load_settings as load_settings_rust  # type: ignore[import-untyped]

from ark_resolver import ark
from ark_resolver.ark_url import ArkUrlFormatter as PythonArkUrlFormatter
from ark_resolver.ark_url import ArkUrlInfo as PythonArkUrlInfo
from ark_resolver.ark_url_rust import ArkUrlFormatter as RustArkUrlFormatter
from ark_resolver.ark_url_rust import ArkUrlInfo as RustArkUrlInfo


@pytest.fixture(scope="module")
def python_settings():
    os.environ["ARK_REGISTRY"] = "tests/ark-registry.ini"
    return ark.load_settings()


@pytest.fixture(scope="module")
def rust_settings():
    os.environ["ARK_REGISTRY"] = "tests/ark-registry.ini"
    return load_settings_rust()


CONVERTIBLE = pytest.mark.parametrize(
    "ark_id",
    [
        "ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn",
        "ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn.20180604T085622513Z",
        "ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn.20180604T085622Z",
        "ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn/pLlW4ODASumZfZFbJdpw1gu.20180604T085622Z",
        "ark:/00000/0002-779b9990a0c3f-6e",
        "ark:/00000/0002-779b9990a0c3f-6e.20190129",
        "ark:/00000/080e-76bb2132d30d6-0",
        "ark:/00000/080e-76bb2132d30d6-0.20190129",
        "ark:/00000/080e-76bb2132d30d6-0.2019111",
    ],
    ids=[
        "resource",
        "resource-ts-fractional",
        "resource-ts-no-fractional",
        "value-with-ts",
        "v0-salsah",
        "v0-salsah-ts",
        "v0-salsah-lowercase",
        "v0-salsah-lowercase-ts",
        "v0-salsah-short-ts",
    ],
)


@CONVERTIBLE
def test_resource_iri_parity(python_settings, rust_settings, ark_id):
    python_iri = PythonArkUrlInfo(python_settings, ark_id).to_resource_iri()
    rust_iri = RustArkUrlInfo(rust_settings, ark_id).to_resource_iri()
    assert python_iri == rust_iri, f"to_resource_iri mismatch for {ark_id}:\n  Python: {python_iri}\n  Rust:   {rust_iri}"


@CONVERTIBLE
def test_timestamp_parity(python_settings, rust_settings, ark_id):
    python_ts = PythonArkUrlInfo(python_settings, ark_id).get_timestamp()
    rust_ts = RustArkUrlInfo(rust_settings, ark_id).get_timestamp()
    assert python_ts == rust_ts, f"get_timestamp mismatch for {ark_id}:\n  Python: {python_ts}\n  Rust:   {rust_ts}"


@CONVERTIBLE
def test_converted_ark_id_parity(python_settings, rust_settings, ark_id):
    """The full chain the /convert route runs, end to end."""
    python_info = PythonArkUrlInfo(python_settings, ark_id)
    python_ark = PythonArkUrlFormatter(python_settings).resource_iri_to_ark_id(
        resource_iri=python_info.to_resource_iri(), timestamp=python_info.get_timestamp()
    )
    rust_info = RustArkUrlInfo(rust_settings, ark_id)
    rust_ark = RustArkUrlFormatter(rust_settings).resource_iri_to_ark_id(
        resource_iri=rust_info.to_resource_iri(), timestamp=rust_info.get_timestamp()
    )
    assert python_ark == rust_ark, f"convert mismatch for {ark_id}:\n  Python: {python_ark}\n  Rust:   {rust_ark}"


@pytest.mark.parametrize(
    "ark_id",
    [
        "ark:/00000/1/ZZZZ/cmfk1DMHRBiR4=_6HXpEFAn",
        "ark:/00000/1/0001/cmfk1DMHRBir4=_6HXpEFAn",
        "ark:/00000/1/0003",
        "ark:/00000/1",
    ],
    ids=[
        "unknown-project",
        "bad-check-digit",
        "project-has-no-resource-iri",
        "top-level-has-no-resource-iri",
    ],
)
def test_convert_error_parity(python_settings, rust_settings, ark_id):
    """An ARK that cannot be converted fails in both implementations."""
    python_error = None
    rust_error = None
    try:
        PythonArkUrlInfo(python_settings, ark_id).to_resource_iri()
    except Exception as e:  # noqa: BLE001
        python_error = type(e).__name__
    try:
        RustArkUrlInfo(rust_settings, ark_id).to_resource_iri()
    except Exception as e:  # noqa: BLE001
        rust_error = type(e).__name__
    assert python_error is not None, f"Python converted {ark_id} instead of rejecting it"
    assert rust_error is not None, f"Rust converted {ark_id} instead of rejecting it"

