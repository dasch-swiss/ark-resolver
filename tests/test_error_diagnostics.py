# Copyright © 2015 - 2025 Swiss National Data and Service Center for the Humanities and/or DaSCH Service Platform contributors.
# SPDX-License-Identifier: Apache-2.0

"""Tests for structured error diagnostics module."""

from http import HTTPStatus

from ark_resolver.ark_url import ArkUrlException
from ark_resolver.ark_url import VersionMismatchException
from ark_resolver.ark_url import VersionZeroNotAllowedException
from ark_resolver.check_digit import CheckDigitException
from ark_resolver.error_diagnostics import ArkErrorCode
from ark_resolver.error_diagnostics import ArkErrorDiagnostic
from ark_resolver.error_diagnostics import classify_exception
from ark_resolver.error_diagnostics import diagnose_non_ark_path
from ark_resolver.error_diagnostics import pre_validate_ark

# ──────────────────────────────────────────────────────────────────────
# Pre-validation: HTML entity corruption
# ──────────────────────────────────────────────────────────────────────


class TestPreValidateHtmlEntities:
    def test_detects_lt_with_semicolon(self):
        diag = pre_validate_ark("ark:/72163/1/080E/Yw_JcLphScqZSPBFNJxtYg&lt;")
        assert diag is not None
        assert diag.code == ArkErrorCode.HTML_ENTITY_CORRUPTION
        assert diag.http_status == HTTPStatus.BAD_REQUEST
        assert "&lt;" in diag.detail  # type: ignore[operator]

    def test_detects_lt_without_semicolon(self):
        diag = pre_validate_ark("ark:/72163/1/080E/Yw_JcLphScqZSPBFNJxtYg&lt")
        assert diag is not None
        assert diag.code == ArkErrorCode.HTML_ENTITY_CORRUPTION

    def test_detects_quot(self):
        diag = pre_validate_ark("ark:/72163/1/080E/pCnQUjyjTri76SnoZXrmogY&quot")
        assert diag is not None
        assert diag.code == ArkErrorCode.HTML_ENTITY_CORRUPTION
        assert "copied from HTML" in diag.message

    def test_detects_amp(self):
        diag = pre_validate_ark("ark:/72163/1/080E/test&amp;more")
        assert diag is not None
        assert diag.code == ArkErrorCode.HTML_ENTITY_CORRUPTION

    def test_hint_contains_cleaned_version(self):
        diag = pre_validate_ark("ark:/72163/1/080E/Yw_JcLphScqZSPBFNJxtYg&lt;")
        assert diag is not None
        assert "ark:/72163/1/080E/Yw_JcLphScqZSPBFNJxtYg<" in diag.hint


# ──────────────────────────────────────────────────────────────────────
# Pre-validation: Trailing junk
# ──────────────────────────────────────────────────────────────────────


class TestPreValidateTrailingJunk:
    def test_detects_trailing_slash(self):
        diag = pre_validate_ark("ark:/72163/1/080E/")
        assert diag is not None
        assert diag.code == ArkErrorCode.TRAILING_JUNK
        assert diag.http_status == HTTPStatus.BAD_REQUEST

    def test_detects_trailing_dot(self):
        diag = pre_validate_ark("ark:/72163/1/0869.")
        assert diag is not None
        assert diag.code == ArkErrorCode.TRAILING_JUNK
        assert "ark:/72163/1/0869" in diag.hint

    def test_detects_trailing_comma(self):
        diag = pre_validate_ark("ark:/72163/1/0001/someUuidHere12345678n,")
        assert diag is not None
        assert diag.code == ArkErrorCode.TRAILING_JUNK

    def test_detects_trailing_multiple_chars(self):
        diag = pre_validate_ark("ark:/72163/1/080E/test./")
        assert diag is not None
        assert diag.code == ArkErrorCode.TRAILING_JUNK


# ──────────────────────────────────────────────────────────────────────
# Pre-validation: UUID length checks
# ──────────────────────────────────────────────────────────────────────


class TestPreValidateUuidLength:
    def test_detects_truncated_uuid_16_chars(self):
        """UUID with only 16 chars (like real Sentry error ARK-RESOLVER-1BB)."""
        diag = pre_validate_ark("ark:/72163/1/0105/bHuHkm9FQ767zrFy")
        assert diag is not None
        assert diag.code == ArkErrorCode.UUID_TOO_SHORT
        assert diag.http_status == HTTPStatus.BAD_REQUEST
        assert "16" in diag.message
        assert "23" in diag.message
        assert "truncated" in diag.hint

    def test_detects_uuid_missing_check_digit_22_chars(self):
        """UUID with 22 chars is missing its check digit."""
        diag = pre_validate_ark("ark:/72163/1/080E/Yw_JcLphScqZSPBFNJxtYg")
        assert diag is not None
        assert diag.code == ArkErrorCode.UUID_TOO_SHORT
        assert "22" in diag.message

    def test_valid_uuid_23_chars_passes(self):
        """23 chars is the expected length — pre-validation should pass."""
        diag = pre_validate_ark("ark:/72163/1/0001/cmfk1DMHRBiR4=_6HXpEFAn")
        assert diag is None

    def test_detects_excessively_long_uuid(self):
        """UUID way too long indicates something appended."""
        long_uuid = "A" * 60
        diag = pre_validate_ark(f"ark:/72163/1/0001/{long_uuid}")
        assert diag is not None
        assert diag.code == ArkErrorCode.UUID_TOO_LONG

    def test_slightly_long_uuid_passes_pre_validation(self):
        """UUIDs slightly over 23 chars pass pre-validation (could be value path or timestamp)."""
        diag = pre_validate_ark("ark:/72163/1/0001/cmfk1DMHRBiR4=_6HXpEFAnXX")
        assert diag is None


# ──────────────────────────────────────────────────────────────────────
# Pre-validation: V0 ARKs and clean inputs
# ──────────────────────────────────────────────────────────────────────


class TestPreValidateCleanInputs:
    def test_v0_ark_passes(self):
        """V0 ARKs don't have UUID format — pre-validation should not flag them."""
        diag = pre_validate_ark("ark:/72163/080c-779b9990a0c3f-6e")
        assert diag is None

    def test_bare_ark_prefix_with_naan_passes(self):
        """Just NAAN without project — pre-validation should pass (let parser handle it)."""
        diag = pre_validate_ark("ark:/72163/1")
        assert diag is None

    def test_completely_valid_v1_ark_passes(self):
        diag = pre_validate_ark("ark:/72163/1/0001/cmfk1DMHRBiR4=_6HXpEFAn")
        assert diag is None


# ──────────────────────────────────────────────────────────────────────
# Exception classification: KeyError (unknown project)
# ──────────────────────────────────────────────────────────────────────


class TestClassifyKeyError:
    def test_key_error_produces_unknown_project(self):
        diag = classify_exception(KeyError("0113"), "ark:/72163/1/0113/someUuid")
        assert diag.code == ArkErrorCode.UNKNOWN_PROJECT
        assert diag.http_status == HTTPStatus.NOT_FOUND
        assert "0113" in diag.message
        assert "0113" in diag.detail  # type: ignore[operator]

    def test_key_error_strips_quotes(self):
        diag = classify_exception(KeyError("'0113'"), "ark:/72163/1/0113/someUuid")
        assert diag.detail == "0113"

    def test_key_error_hint_mentions_environment(self):
        diag = classify_exception(KeyError("0113"), "ark:/72163/1/0113/someUuid")
        assert "environment" in diag.hint


# ──────────────────────────────────────────────────────────────────────
# Exception classification: CheckDigitException
# ──────────────────────────────────────────────────────────────────────


class TestClassifyCheckDigitException:
    def test_check_digit_exception(self):
        diag = classify_exception(
            CheckDigitException("Invalid base64url character: '!'"),
            "ark:/72163/1/080E/test!invalid",
        )
        assert diag.code == ArkErrorCode.INVALID_CHECK_DIGIT
        assert diag.http_status == HTTPStatus.BAD_REQUEST
        assert "check digit" in diag.hint.lower()


# ──────────────────────────────────────────────────────────────────────
# Exception classification: ArkUrlException variants
# ──────────────────────────────────────────────────────────────────────


class TestClassifyArkUrlException:
    def test_version_0_not_allowed(self):
        diag = classify_exception(
            VersionZeroNotAllowedException("Invalid ARK ID (version 0 not allowed): ark:/72163/080c-abc-def"),
            "ark:/72163/080c-abc-def",
        )
        assert diag.code == ArkErrorCode.VERSION_0_NOT_ALLOWED
        assert diag.http_status == HTTPStatus.UNPROCESSABLE_ENTITY
        assert "version 1" in diag.hint.lower()

    def test_version_mismatch(self):
        diag = classify_exception(
            VersionMismatchException(
                "Invalid ARK ID ark:/72163/99. The version of the ARK ID doesn't match the version defined in the settings."
            ),
            "ark:/72163/99",
        )
        assert diag.code == ArkErrorCode.VERSION_MISMATCH
        assert diag.http_status == HTTPStatus.UNPROCESSABLE_ENTITY

    def test_generic_invalid_ark(self):
        diag = classify_exception(
            ArkUrlException("Invalid ARK ID: ark:/72163/1/080E/badstuff"),
            "ark:/72163/1/080E/badstuff",
        )
        assert diag.code == ArkErrorCode.MALFORMED_ARK
        assert diag.http_status == HTTPStatus.BAD_REQUEST
        assert "Expected format" in diag.hint

    def test_unexpected_exception_type(self):
        diag = classify_exception(
            RuntimeError("something unexpected"),
            "ark:/72163/1/0001/test",
        )
        assert diag.code == ArkErrorCode.MALFORMED_ARK
        assert diag.http_status == HTTPStatus.BAD_REQUEST


# ──────────────────────────────────────────────────────────────────────
# Non-ARK path diagnostic
# ──────────────────────────────────────────────────────────────────────


class TestDiagnoseNonArkPath:
    def test_favicon(self):
        diag = diagnose_non_ark_path("favicon.ico")
        assert diag.code == ArkErrorCode.NOT_AN_ARK
        assert diag.http_status == HTTPStatus.BAD_REQUEST
        assert "favicon.ico" in diag.message
        assert "ark:/" in diag.hint

    def test_random_path(self):
        diag = diagnose_non_ark_path("some/random/path")
        assert diag.code == ArkErrorCode.NOT_AN_ARK


# ──────────────────────────────────────────────────────────────────────
# Serialization: to_dict()
# ──────────────────────────────────────────────────────────────────────


class TestSerialization:
    def test_to_dict_includes_required_fields(self):
        diag = ArkErrorDiagnostic(
            code=ArkErrorCode.UNKNOWN_PROJECT,
            message="Project '0113' is not registered.",
            hint="Check the project shortcode.",
            ark_id="ark:/72163/1/0113/test",
            http_status=HTTPStatus.NOT_FOUND,
            detail="0113",
        )
        d = diag.to_dict()
        assert "error" in d
        assert d["error"]["code"] == "UNKNOWN_PROJECT"
        assert d["error"]["message"] == "Project '0113' is not registered."
        assert d["error"]["hint"] == "Check the project shortcode."
        assert d["error"]["ark_id"] == "ark:/72163/1/0113/test"
        assert d["error"]["detail"] == "0113"

    def test_to_dict_omits_detail_when_none(self):
        diag = ArkErrorDiagnostic(
            code=ArkErrorCode.MALFORMED_ARK,
            message="Bad ARK",
            hint="Fix it",
            ark_id="ark:/bad",
            http_status=HTTPStatus.BAD_REQUEST,
        )
        d = diag.to_dict()
        assert "detail" not in d["error"]

    def test_error_code_is_string(self):
        """Error code should be a plain string in the dict, not an Enum repr."""
        diag = ArkErrorDiagnostic(
            code=ArkErrorCode.HTML_ENTITY_CORRUPTION,
            message="test",
            hint="test",
            ark_id="test",
            http_status=HTTPStatus.BAD_REQUEST,
        )
        d = diag.to_dict()
        assert isinstance(d["error"]["code"], str)
        assert d["error"]["code"] == "HTML_ENTITY_CORRUPTION"
