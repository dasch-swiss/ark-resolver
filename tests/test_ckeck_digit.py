from ark_resolver import check_digit as ckeck_digit_py

def test_base64url_check_digit():
    correct_resource_id = "cmfk1DMHRBiR4-_6HXpEFA"

    # reject a string without a check digit
    assert not ckeck_digit_py.is_valid(correct_resource_id)

    # calculate a check digit for a string and validate it
    correct_resource_id_check_digit = "n"
    check_digit = ckeck_digit_py.calculate_check_digit(correct_resource_id)
    assert check_digit == correct_resource_id_check_digit
    correct_resource_id_with_correct_check_digit = correct_resource_id + check_digit
    assert ckeck_digit_py.is_valid(correct_resource_id_with_correct_check_digit)

    # reject a string with an incorrect check digit
    correct_resource_id_with_incorrect_check_digit = correct_resource_id + "m"
    assert not ckeck_digit_py.is_valid(correct_resource_id_with_incorrect_check_digit)

    # reject a string with a missing character
    resource_id_with_missing_character = "cmfk1DMHRBiR4-6HXpEFA"
    resource_id_with_missing_character_and_correct_check_digit = resource_id_with_missing_character + correct_resource_id_check_digit
    assert not ckeck_digit_py.is_valid(resource_id_with_missing_character_and_correct_check_digit)

    # reject a string with an incorrect character
    resource_id_with_incorrect_character = "cmfk1DMHRBir4-_6HXpEFA"
    resource_id_with_incorrect_character_and_correct_check_digit = resource_id_with_incorrect_character + correct_resource_id_check_digit
    assert not ckeck_digit_py.is_valid(resource_id_with_incorrect_character_and_correct_check_digit)

    # reject a string with swapped characters
    resource_id_with_swapped_characters = "cmfk1DMHRBiR4_-6HXpEFA"
    resource_id_with_swapped_characters_and_correct_check_digit = resource_id_with_swapped_characters + correct_resource_id_check_digit
    assert not ckeck_digit_py.is_valid(resource_id_with_swapped_characters_and_correct_check_digit)
