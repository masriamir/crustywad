#![no_main]
use libfuzzer_sys::fuzz_target;

use crustywad::ParseOptions;
use crustywad::map::strife::{DEMO_DIALOG_RECORD_SIZE, RETAIL_DIALOG_RECORD_SIZE, parse_dialogue};

fuzz_target!(|data: &[u8]| {
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        match parse_dialogue(data, &options) {
            Ok((records, _format, _warnings)) => {
                // O(input) allocation invariant (ADR-0016 §1): every record
                // consumes at least DEMO_DIALOG_RECORD_SIZE input bytes.
                assert!(
                    records.len() * DEMO_DIALOG_RECORD_SIZE
                        <= data.len().max(DEMO_DIALOG_RECORD_SIZE),
                    "{} records exceed the O(input) bound for {} bytes",
                    records.len(),
                    data.len()
                );
                // Strict success implies an exact record multiple.
                if options.strictness == crustywad::Strictness::Strict {
                    assert!(
                        data.len().is_multiple_of(RETAIL_DIALOG_RECORD_SIZE)
                            || data.len().is_multiple_of(DEMO_DIALOG_RECORD_SIZE)
                    );
                }
            }
            Err(_) => {
                // Lenient mode only errs through the defensive decode arm,
                // which no byte input should reach: lenient always truncates
                // to a valid multiple first.
                assert!(
                    options.strictness == crustywad::Strictness::Strict,
                    "lenient parse_dialogue returned Err on {} bytes",
                    data.len()
                );
            }
        }
    }
});
