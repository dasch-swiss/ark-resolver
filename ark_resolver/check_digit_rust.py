#!/usr/bin/env python3

# Copyright © 2021 - 2025 Swiss National Data and Service Center for the Humanities and/or DaSCH Service Platform contributors.
# SPDX-License-Identifier: Apache-2.0

"""
Rust-backed implementation of check digit functions for base64url-encoded IDs.
This module provides the same interface as check_digit.py but uses Rust implementations
for improved performance.
"""

from ark_resolver._rust import (
    calculate_check_digit as _calculate_check_digit,
    calculate_modulus as _calculate_modulus,
    is_valid as _is_valid,
    to_check_digit as _to_check_digit,
    to_int as _to_int,
    weighted_value as _weighted_value,
)


class CheckDigitException(Exception):
    def __init__(self, message: str) -> None:
        self.message = message


def is_valid(code: str) -> bool:
    """Checks whether a code with a check digit is valid."""
    return _is_valid(code)


def calculate_check_digit(code: str) -> str:
    """Calculates the check digit for a code."""
    return _calculate_check_digit(code)


def calculate_modulus(code: str, includes_check_digit: bool) -> int:
    """Calculates the modulus for a code."""
    return _calculate_modulus(code, includes_check_digit)


def weighted_value(char_value: int, right_pos: int) -> int:
    """Calculates the weighted value of a character in the code at a specified position."""
    return _weighted_value(char_value, right_pos)


def to_int(char: str) -> int:
    """Converts a character at a specified position to an integer value."""
    if len(char) != 1:
        raise CheckDigitException(f"Expected single character, got: '{char}'")
    return _to_int(char)


def to_check_digit(char_value: int) -> str:
    """Converts an integer value to a check digit."""
    return _to_check_digit(char_value)