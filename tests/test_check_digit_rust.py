from ark_resolver import check_digit_rust as check_digit_rust_py


def test_base64url_check_digit_rust():
    correct_resource_id = "cmfk1DMHRBiR4-_6HXpEFA"

    # reject a string without a check digit
    assert not check_digit_rust_py.is_valid(correct_resource_id)

    # calculate a check digit for a string and validate it
    correct_resource_id_check_digit = "n"
    check_digit = check_digit_rust_py.calculate_check_digit(correct_resource_id)
    assert check_digit == correct_resource_id_check_digit
    correct_resource_id_with_correct_check_digit = correct_resource_id + check_digit
    assert check_digit_rust_py.is_valid(correct_resource_id_with_correct_check_digit)

    # reject a string with an incorrect check digit
    correct_resource_id_with_incorrect_check_digit = correct_resource_id + "m"
    assert not check_digit_rust_py.is_valid(correct_resource_id_with_incorrect_check_digit)

    # reject a string with a missing character
    resource_id_with_missing_character = "cmfk1DMHRBiR4-6HXpEFA"
    resource_id_with_missing_character_and_correct_check_digit = resource_id_with_missing_character + correct_resource_id_check_digit
    assert not check_digit_rust_py.is_valid(resource_id_with_missing_character_and_correct_check_digit)

    # reject a string with an incorrect character
    resource_id_with_incorrect_character = "cmfk1DMHRBir4-_6HXpEFA"
    resource_id_with_incorrect_character_and_correct_check_digit = resource_id_with_incorrect_character + correct_resource_id_check_digit
    assert not check_digit_rust_py.is_valid(resource_id_with_incorrect_character_and_correct_check_digit)

    # reject a string with swapped characters
    resource_id_with_swapped_characters = "cmfk1DMHRBiR4_-6HXpEFA"
    resource_id_with_swapped_characters_and_correct_check_digit = resource_id_with_swapped_characters + correct_resource_id_check_digit
    assert not check_digit_rust_py.is_valid(resource_id_with_swapped_characters_and_correct_check_digit)


def test_rust_vs_python_parity():
    """Test that Rust and Python implementations produce identical results."""
    from ark_resolver import check_digit as check_digit_py

    test_codes = [
        "cmfk1DMHRBiR4-_6HXpEFA",
        "ABC123",
        "hello-world_test",
        "0123456789",
        "abcdefghijklmnopqrstuvwxyz",
        "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
        "-_",
    ]

    for code in test_codes:
        # Test calculate_check_digit parity
        py_check_digit = check_digit_py.calculate_check_digit(code)
        rust_check_digit = check_digit_rust_py.calculate_check_digit(code)
        assert py_check_digit == rust_check_digit, (
            f"Check digit mismatch for '{code}': Python='{py_check_digit}', Rust='{rust_check_digit}'"
        )

        # Test is_valid parity for code without check digit
        py_valid_no_check = check_digit_py.is_valid(code)
        rust_valid_no_check = check_digit_rust_py.is_valid(code)
        assert py_valid_no_check == rust_valid_no_check, (
            f"is_valid mismatch for '{code}' (no check): Python={py_valid_no_check}, Rust={rust_valid_no_check}"
        )

        # Test is_valid parity for code with check digit
        code_with_check = code + py_check_digit
        py_valid_with_check = check_digit_py.is_valid(code_with_check)
        rust_valid_with_check = check_digit_rust_py.is_valid(code_with_check)
        assert py_valid_with_check == rust_valid_with_check, (
            f"is_valid mismatch for '{code_with_check}': Python={py_valid_with_check}, Rust={rust_valid_with_check}"
        )

        # Test calculate_modulus parity
        py_modulus_no_check = check_digit_py.calculate_modulus(code, False)
        rust_modulus_no_check = check_digit_rust_py.calculate_modulus(code, False)
        assert py_modulus_no_check == rust_modulus_no_check, (
            f"calculate_modulus mismatch for '{code}' (no check): Python={py_modulus_no_check}, Rust={rust_modulus_no_check}"
        )

        py_modulus_with_check = check_digit_py.calculate_modulus(code_with_check, True)
        rust_modulus_with_check = check_digit_rust_py.calculate_modulus(code_with_check, True)
        assert py_modulus_with_check == rust_modulus_with_check, (
            f"calculate_modulus mismatch for '{code_with_check}' (with check): " +
            f"Python={py_modulus_with_check}, Rust={rust_modulus_with_check}"
        )


def test_helper_functions():
    """Test helper functions for parity."""
    from ark_resolver import check_digit as check_digit_py

    # Test to_int
    test_chars = ["A", "Z", "a", "z", "0", "9", "-", "_"]
    for char in test_chars:
        py_result = check_digit_py.to_int(char)
        rust_result = check_digit_rust_py.to_int(char)
        assert py_result == rust_result, f"to_int mismatch for '{char}': Python={py_result}, Rust={rust_result}"

    # Test to_check_digit
    test_values = [0, 25, 26, 51, 52, 61, 62, 63]
    for value in test_values:
        py_result = check_digit_py.to_check_digit(value)
        rust_result = check_digit_rust_py.to_check_digit(value)
        assert py_result == rust_result, f"to_check_digit mismatch for {value}: Python='{py_result}', Rust='{rust_result}'"

    # Test weighted_value
    test_cases = [(5, 3), (0, 10), (63, 1), (1, 63)]
    for char_value, right_pos in test_cases:
        py_result = check_digit_py.weighted_value(char_value, right_pos)
        rust_result = check_digit_rust_py.weighted_value(char_value, right_pos)
        assert py_result == rust_result, f"weighted_value mismatch for ({char_value}, {right_pos}): Python={py_result}, Rust={rust_result}"
