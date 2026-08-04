#![no_main]
use libfuzzer_sys::fuzz_target;

use crustywad::ParseOptions;
use crustywad::map::strife::{
    DEMO_DIALOG_RECORD_SIZE, DialogueFormat, RETAIL_DIALOG_RECORD_SIZE, parse_dialogue,
};

fuzz_target!(|data: &[u8]| {
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        match parse_dialogue(data, &options) {
            Ok((records, format, _warnings)) => {
                // O(input) allocation invariant (ADR-0016 §1): every record
                // consumes at least its format's full record size of input.
                // Division keeps the bound tight (a record materialized from
                // fewer input bytes than its record size fails) and cannot
                // overflow, unlike a records * size multiplication.
                let record_size = match format {
                    DialogueFormat::Retail => RETAIL_DIALOG_RECORD_SIZE,
                    DialogueFormat::Demo => DEMO_DIALOG_RECORD_SIZE,
                };
                assert!(
                    records.len() <= data.len() / record_size,
                    "{} {format:?} records exceed the O(input) bound for {} bytes",
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
