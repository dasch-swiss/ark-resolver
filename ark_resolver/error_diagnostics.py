# Copyright © 2015 - 2025 Swiss National Data and Service Center for the Humanities and/or DaSCH Service Platform contributors.
# SPDX-License-Identifier: Apache-2.0

"""
Structured, agent-friendly error diagnostics for ARK resolution failures.

BR: Every error response must include a machine-readable code, a human message,
and an actionable hint so that agents and humans can understand what went wrong
and how to fix it.
"""

import re
from dataclasses import dataclass
from enum import Enum

import sentry_sdk
from sanic import HTTPResponse
from sanic import json as sanic_json

from ark_resolver.ark_url import ArkUrlException
from ark_resolver.ark_url import VersionMismatchException
from ark_resolver.ark_url import VersionZeroNotAllowedException
from ark_resolver.check_digit import CheckDigitException

# BR: Expected UUID length is 22 base64url characters + 1 check digit = 23
EXPECTED_UUID_LENGTH = 23

# BR: Detect HTML entity corruption before parsing to provide specific fix guidance
_HTML_ENTITIES = re.compile(r"&(lt|gt|amp|quot|#\d+|#x[0-9a-fA-F]+);?")

# BR: Detect trailing non-ARK characters that indicate copy-paste errors
_TRAILING_JUNK = re.compile(r"[./,;:!?\s]+$")

# BR: V1 ARK path structure for extracting UUID portions for length/character checks
# Captures: first UUID (resource_id), optional second UUID (value_id)
_V1_UUID_EXTRACTOR = re.compile(r"^ark:/[0-9]+/[0-9]+/[0-9A-Fa-f]{4}/([^\s/.]+)")

_BASE64URL_VALID = re.compile(r"^[A-Za-z0-9_=]+$")


class ArkErrorCode(str, Enum):
    """Machine-readable error codes for ARK resolution failures."""

    NOT_AN_ARK = "NOT_AN_ARK"
    HTML_ENTITY_CORRUPTION = "HTML_ENTITY_CORRUPTION"
    TRAILING_JUNK = "TRAILING_JUNK"
    INVALID_UUID_CHARS = "INVALID_UUID_CHARS"
    UUID_TOO_SHORT = "UUID_TOO_SHORT"
    UUID_TOO_LONG = "UUID_TOO_LONG"
    INVALID_CHECK_DIGIT = "INVALID_CHECK_DIGIT"
    UNKNOWN_PROJECT = "UNKNOWN_PROJECT"
    VERSION_0_NOT_ALLOWED = "VERSION_0_NOT_ALLOWED"
    VERSION_MISMATCH = "VERSION_MISMATCH"
    MALFORMED_ARK = "MALFORMED_ARK"


@dataclass(frozen=True)
class ArkErrorDiagnostic:
    """
    Structured diagnostic for an ARK resolution failure.

    BR: Every error response must include a machine-readable code, a human message,
    and an actionable hint so agents can programmatically detect and fix common issues.
    """

    code: ArkErrorCode
    message: str
    hint: str
    ark_id: str
    http_status: int
    detail: str | None = None

    def to_dict(self) -> dict:
        result: dict = {
            "error": {
                "code": self.code.value,
                "message": self.message,
                "hint": self.hint,
                "ark_id": self.ark_id,
            }
        }
        if self.detail is not None:
            result["error"]["detail"] = self.detail
        return result


_ENTITY_REPLACEMENTS = {"lt": "<", "gt": ">", "amp": "&", "quot": '"'}


def _replace_entity(m: re.Match) -> str:
    """BR: Resolve HTML entities (named and numeric) to their character equivalents for hint display."""
    name = m.group(1)
    if name in _ENTITY_REPLACEMENTS:
        return _ENTITY_REPLACEMENTS[name]
    if name.startswith("#x"):
        return chr(int(name[2:], 16))
    if name.startswith("#"):
        return chr(int(name[1:]))
    return ""


def _clean_html_entities(ark_id: str) -> str:
    return _HTML_ENTITIES.sub(_replace_entity, ark_id)


def pre_validate_ark(ark_id: str) -> ArkErrorDiagnostic | None:
    """
    BR: Pre-validate ARK ID for common corruption patterns before parsing.
    Returns an ArkErrorDiagnostic if corruption is detected, None if the input looks clean.

    Check ordering matters: HTML entities must be checked before trailing junk because
    semicolons (;) in _TRAILING_JUNK would incorrectly match entity suffixes like &lt;
    """
    # BR: HTML entities indicate the ARK was copied from HTML source code rather than rendered text
    entity_match = _HTML_ENTITIES.search(ark_id)
    if entity_match:
        entity = entity_match.group(0)
        cleaned = _clean_html_entities(ark_id)
        return ArkErrorDiagnostic(
            code=ArkErrorCode.HTML_ENTITY_CORRUPTION,
            message=f"ARK ID contains HTML entity '{entity}'. It appears to have been copied from HTML source code.",
            hint=f"Remove HTML entities from the ARK ID. The cleaned version might be: {cleaned}",
            ark_id=ark_id,
            http_status=400,
            detail=entity,
        )

    # BR: Trailing punctuation indicates copy-paste errors (e.g., period from end of sentence)
    junk_match = _TRAILING_JUNK.search(ark_id)
    if junk_match:
        junk = junk_match.group(0).strip()
        cleaned = ark_id[: junk_match.start()]
        return ArkErrorDiagnostic(
            code=ArkErrorCode.TRAILING_JUNK,
            message=f"ARK ID has trailing character(s) '{junk}' that are not part of a valid ARK.",
            hint=f"Remove the trailing '{junk}'. Try: {cleaned}",
            ark_id=ark_id,
            http_status=400,
            detail=junk,
        )

    # BR: For V1 ARKs, check UUID portion for character validity and length to catch truncation early
    uuid_match = _V1_UUID_EXTRACTOR.match(ark_id)
    if uuid_match:
        uuid_part = uuid_match.group(1)

        if not _BASE64URL_VALID.match(uuid_part):
            bad_chars = sorted({c for c in uuid_part if not re.match(r"[A-Za-z0-9_=]", c)})
            return ArkErrorDiagnostic(
                code=ArkErrorCode.INVALID_UUID_CHARS,
                message=f"Resource identifier contains invalid character(s): {', '.join(repr(c) for c in bad_chars)}.",
                hint=(
                    "The resource identifier must use only base64url characters: "
                    "A-Z, a-z, 0-9, underscore (_), and equals (=, which escapes dash)."
                ),
                ark_id=ark_id,
                http_status=400,
                detail=", ".join(repr(c) for c in bad_chars),
            )

        if len(uuid_part) < EXPECTED_UUID_LENGTH:
            return ArkErrorDiagnostic(
                code=ArkErrorCode.UUID_TOO_SHORT,
                message=(
                    f"Resource identifier is {len(uuid_part)} characters, "
                    f"expected {EXPECTED_UUID_LENGTH} (22 base64url chars + 1 check digit)."
                ),
                hint="The ARK ID appears truncated. Ensure the full resource identifier including the check digit character is included.",
                ark_id=ark_id,
                http_status=400,
                detail=f"length={len(uuid_part)}, expected={EXPECTED_UUID_LENGTH}",
            )

        # BR: Allow up to 30 extra chars to avoid false positives on value-path
        # segments or future format extensions; only flag truly pathological lengths
        if len(uuid_part) > EXPECTED_UUID_LENGTH + 30:
            return ArkErrorDiagnostic(
                code=ArkErrorCode.UUID_TOO_LONG,
                message=f"Resource identifier is {len(uuid_part)} characters, expected {EXPECTED_UUID_LENGTH}.",
                hint="The resource identifier is too long. It should be exactly 22 base64url characters plus 1 check digit.",
                ark_id=ark_id,
                http_status=400,
                detail=f"length={len(uuid_part)}, expected={EXPECTED_UUID_LENGTH}",
            )

    return None


def classify_exception(
    exception: Exception,
    ark_id: str,
) -> ArkErrorDiagnostic:
    """
    BR: Classify exceptions from ArkUrlInfo parsing into structured diagnostics
    with specific error codes, messages, and hints based on the exception type and content.
    """
    if isinstance(exception, KeyError):
        # BR: Unknown project codes produce a 404 — the ARK structure is valid but the project does not exist in the registry
        missing_project = str(exception).strip("'\"")
        return ArkErrorDiagnostic(
            code=ArkErrorCode.UNKNOWN_PROJECT,
            message=f"Project '{missing_project}' is not registered in the ARK resolver.",
            hint=(
                f"Check that the project shortcode '{missing_project}' is correct. The project may not yet be deployed to this environment."
            ),
            ark_id=ark_id,
            http_status=404,
            detail=missing_project,
        )

    if isinstance(exception, CheckDigitException):
        return ArkErrorDiagnostic(
            code=ArkErrorCode.INVALID_CHECK_DIGIT,
            message=f"Check digit validation failed: {exception.message}",
            hint="The last character of the resource identifier is a check digit. Verify the ARK ID was copied completely and correctly.",
            ark_id=ark_id,
            http_status=400,
        )

    if isinstance(exception, VersionZeroNotAllowedException):
        return ArkErrorDiagnostic(
            code=ArkErrorCode.VERSION_0_NOT_ALLOWED,
            message=exception.message,
            hint=(
                "This project does not accept version 0 (legacy salsah.org) ARK URLs. "
                "Use the version 1 format: ark:/{NAAN}/1/{project_id}/{uuid}{check_digit}"
            ),
            ark_id=ark_id,
            http_status=422,
        )

    if isinstance(exception, VersionMismatchException):
        return ArkErrorDiagnostic(
            code=ArkErrorCode.VERSION_MISMATCH,
            message=exception.message,
            hint="The ARK version number does not match the expected version. The current version is 1.",
            ark_id=ark_id,
            http_status=422,
        )

    if isinstance(exception, ArkUrlException):
        # BR: Generic ArkUrlException — the regex didn't match any known ARK format
        return ArkErrorDiagnostic(
            code=ArkErrorCode.MALFORMED_ARK,
            message=exception.message,
            hint=(
                "Expected format: ark:/{NAAN}/1/{4-hex-project-id}/{base64url-resource-id}{check-digit}"
                " — Example: ark:/72163/1/0001/cmfk1DMHRBiR4=_6HXpEFAn"
            ),
            ark_id=ark_id,
            http_status=400,
        )

    # BR: Unexpected exception types still get a structured response
    return ArkErrorDiagnostic(
        code=ArkErrorCode.MALFORMED_ARK,
        message=f"Unexpected error processing ARK ID: {exception}",
        hint="Expected format: ark:/{NAAN}/1/{4-hex-project-id}/{base64url-resource-id}{check-digit}",
        ark_id=ark_id,
        http_status=400,
    )


def diagnose_non_ark_path(path: str) -> ArkErrorDiagnostic:
    """BR: Paths that don't start with ark:/ are not ARK identifiers."""
    return ArkErrorDiagnostic(
        code=ArkErrorCode.NOT_AN_ARK,
        message=f"Path '{path}' is not an ARK identifier.",
        hint="ARK URLs must start with 'ark:/' followed by the NAAN. Example: ark:/72163/1/0001/{resource-id}{check-digit}",
        ark_id=path,
        http_status=400,
    )


def report_error_to_sentry(
    endpoint: str,
    diagnostic: ArkErrorDiagnostic,
    ark_id_decoded: str,
    exception: Exception | None = None,
) -> None:
    """BR: Report structured error diagnostics to Sentry with granular fingerprints for better issue grouping."""
    with sentry_sdk.push_scope() as scope:
        scope.fingerprint = [endpoint, diagnostic.code.value]
        scope.set_tag("ark_id", ark_id_decoded[:100])
        scope.set_tag("error_code", diagnostic.code.value)
        if diagnostic.detail:
            scope.set_tag("error_detail", diagnostic.detail[:50])
        if exception is not None:
            sentry_sdk.capture_exception(exception)
        else:
            sentry_sdk.capture_message(diagnostic.message, level="error")


def error_response(diagnostic: ArkErrorDiagnostic) -> HTTPResponse:
    """Build a Sanic JSON error response from an ArkErrorDiagnostic."""
    return sanic_json(diagnostic.to_dict(), status=diagnostic.http_status)
