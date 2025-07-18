/// Integration test for the hexagonal architecture core functionality
/// This test verifies that the pure Rust domain and use case layers work without PyO3 dependencies
use ark_resolver::core::domain::check_digit;
use ark_resolver::core::use_cases::check_digit_validator::CheckDigitValidator;

#[test]
fn test_domain_layer_functionality() {
    // Test is_valid
    assert!(check_digit::is_valid("cmfk1DMHRBiR4-_6HXpEFAn").unwrap());
    assert!(!check_digit::is_valid("cmfk1DMHRBiR4-_6HXpEFA").unwrap());
    assert!(!check_digit::is_valid("").unwrap());

    // Test calculate_check_digit
    let check_digit_char = check_digit::calculate_check_digit("cmfk1DMHRBiR4-_6HXpEFA").unwrap();
    assert_eq!(check_digit_char, 'n');

    // Test calculate_modulus
    assert_eq!(
        check_digit::calculate_modulus("cmfk1DMHRBiR4-_6HXpEFAn", true).unwrap(),
        0
    );

    // Test to_int
    assert_eq!(check_digit::to_int('A').unwrap(), 0);
    assert_eq!(check_digit::to_int('Z').unwrap(), 25);
    assert_eq!(check_digit::to_int('a').unwrap(), 26);
    assert_eq!(check_digit::to_int('z').unwrap(), 51);
    assert_eq!(check_digit::to_int('0').unwrap(), 52);
    assert_eq!(check_digit::to_int('9').unwrap(), 61);
    assert_eq!(check_digit::to_int('-').unwrap(), 62);
    assert_eq!(check_digit::to_int('_').unwrap(), 63);

    // Test to_check_digit
    assert_eq!(check_digit::to_check_digit(0).unwrap(), 'A');
    assert_eq!(check_digit::to_check_digit(25).unwrap(), 'Z');
    assert_eq!(check_digit::to_check_digit(26).unwrap(), 'a');
    assert_eq!(check_digit::to_check_digit(51).unwrap(), 'z');
    assert_eq!(check_digit::to_check_digit(52).unwrap(), '0');
    assert_eq!(check_digit::to_check_digit(61).unwrap(), '9');
    assert_eq!(check_digit::to_check_digit(62).unwrap(), '-');
    assert_eq!(check_digit::to_check_digit(63).unwrap(), '_');

    // Test weighted_value
    assert_eq!(check_digit::weighted_value(5, 3), 15);
    assert_eq!(check_digit::weighted_value(0, 10), 0);
}

#[test]
fn test_use_case_layer_functionality() {
    let validator = CheckDigitValidator::new();

    // Test validation
    assert!(validator.is_valid("cmfk1DMHRBiR4-_6HXpEFAn").unwrap());
    assert!(!validator.is_valid("cmfk1DMHRBiR4-_6HXpEFA").unwrap());
    assert!(!validator.is_valid("").unwrap());

    // Test check digit calculation
    let check_digit_char = validator
        .calculate_check_digit("cmfk1DMHRBiR4-_6HXpEFA")
        .unwrap();
    assert_eq!(check_digit_char, 'n');

    // Test modulus calculation
    assert_eq!(
        validator
            .calculate_modulus("cmfk1DMHRBiR4-_6HXpEFAn", true)
            .unwrap(),
        0
    );

    // Test utility functions
    assert_eq!(validator.to_int('A').unwrap(), 0);
    assert_eq!(validator.to_check_digit(0).unwrap(), 'A');
    assert_eq!(validator.weighted_value(5, 3), 15);

    // Test business logic methods
    let code_with_check_digit = validator.add_check_digit("cmfk1DMHRBiR4-_6HXpEFA").unwrap();
    assert_eq!(code_with_check_digit, "cmfk1DMHRBiR4-_6HXpEFAn");

    let code_without_check_digit = validator
        .validate_and_strip_check_digit("cmfk1DMHRBiR4-_6HXpEFAn")
        .unwrap();
    assert_eq!(code_without_check_digit, "cmfk1DMHRBiR4-_6HXpEFA");
}

#[test]
fn test_error_handling() {
    let validator = CheckDigitValidator::new();

    // Test empty code error
    assert!(validator.calculate_check_digit("").is_err());
    assert!(validator.validate_and_strip_check_digit("").is_err());

    // Test invalid character error
    assert!(validator.to_int('@').is_err());

    // Test invalid character value error
    assert!(validator.to_check_digit(-1).is_err());
    assert!(validator.to_check_digit(64).is_err());

    // Test invalid code with wrong check digit
    assert!(validator
        .validate_and_strip_check_digit("cmfk1DMHRBiR4-_6HXpEFAx")
        .is_err());
}

#[test]
fn test_round_trip_consistency() {
    let validator = CheckDigitValidator::new();
    let test_codes = vec!["cmfk1DMHRBiR4-_6HXpEFA", "ABC123", "test-code_42", "Z9-_a"];

    for code in test_codes {
        // Add check digit and verify it makes the code valid
        let code_with_check_digit = validator.add_check_digit(code).unwrap();
        assert!(validator.is_valid(&code_with_check_digit).unwrap());

        // Strip check digit and verify we get back the original code
        let stripped_code = validator
            .validate_and_strip_check_digit(&code_with_check_digit)
            .unwrap();
        assert_eq!(stripped_code, code);
    }
}
