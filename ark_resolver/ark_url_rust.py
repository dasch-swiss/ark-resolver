#!/usr/bin/env python3

# Copyright © 2015 - 2025 Swiss National Data and Service Center for the Humanities and/or DaSCH Service Platform contributors.
# SPDX-License-Identifier: Apache-2.0

# Unused imports removed - functionality moved to Rust

from ark_resolver._rust import ArkUrlFormatter  # type: ignore[import-untyped]

# TODO: the rust module does not seem to be typed in python land.
from ark_resolver._rust import ArkUrlInfo  # type: ignore[import-untyped]
from ark_resolver._rust import ArkUrlSettings  # type: ignore[import-untyped]
from ark_resolver._rust import add_check_digit_and_escape as rust_add_check_digit_and_escape  # type: ignore[import-untyped]
from ark_resolver._rust import unescape_and_validate_uuid as rust_unescape_and_validate_uuid  # type: ignore[import-untyped]
from ark_resolver.ark_url import ArkUrlException

#################################################################################################
# Tools for generating and parsing DSP ARK URLs.
TIMESTAMP_LENGTH = 8

# Explicit exports to indicate these imports are the public API
__all__ = [
    "ArkUrlException",
    "ArkUrlFormatter",
    "ArkUrlInfo",
    "ArkUrlSettings",
    "add_check_digit_and_escape",
    "unescape_and_validate_uuid",
]


# ArkUrlInfo is now implemented in Rust and imported above


def add_check_digit_and_escape(uuid: str) -> str:
    """
    Adds a check digit to a Base64-encoded UUID, and escapes the result.
    Uses the Rust implementation for performance.
    """
    return rust_add_check_digit_and_escape(uuid)


def unescape_and_validate_uuid(ark_url: str, escaped_uuid: str) -> str:
    """
    Unescapes a Base64-encoded UUID, validates its check digit, and returns the unescaped UUID without the check digit.
    Uses the Rust implementation for performance.
    """
    try:
        return rust_unescape_and_validate_uuid(ark_url, escaped_uuid)
    except ValueError as e:
        # Convert Rust ValueError to ArkUrlException for API compatibility
        raise ArkUrlException(str(e))


# ArkUrlFormatter is now implemented in Rust and imported above
# The Rust implementation provides exact API compatibility with the original Python version

# Export ArkUrlException for API compatibility
__all__ = ["ArkUrlException", "ArkUrlFormatter", "ArkUrlInfo", "add_check_digit_and_escape", "unescape_and_validate_uuid"]
