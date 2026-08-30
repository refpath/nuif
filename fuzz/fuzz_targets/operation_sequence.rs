#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|choices: &[u8]| {
    if choices.len() > nuif_testing::MAX_OPERATION_CHOICE_BYTES {
        return;
    }
    assert!(
        nuif_testing::verify_operation_choices(choices).is_ok(),
        "typed operation choice sequence violated a conformance relation"
    );
});
