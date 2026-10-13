#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Fuzzing the parsers: arbitrary input must never panic.

use proptest::prelude::*;
use rustfig_format::{read_any, read_fig, write_fig};

const SAMPLE: &[u8] = include_bytes!("sample.fig");

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn random_bytes_never_panic(data in proptest::collection::vec(any::<u8>(), 0..2048)) {
        let _ = read_any(&data);
        let mut with_header = b"#FIG 3.2\n".to_vec();
        with_header.extend_from_slice(&data);
        let _ = read_fig(&with_header);
    }

    #[test]
    fn mutated_sample_never_panics(pos in 0usize..2000, byte in any::<u8>(), cut in 0usize..2000) {
        let mut data = SAMPLE.to_vec();
        if let Some(b) = data.get_mut(pos % SAMPLE.len()) { *b = byte; }
        data.truncate(cut.max(10));
        if let Ok(doc) = read_fig(&data) {
            // Whatever parses must write and parse again.
            let out = write_fig(&doc);
            read_fig(out.as_bytes()).unwrap();
        }
    }

    #[test]
    fn token_soup(tokens in proptest::collection::vec(prop_oneof![
        Just("1".to_string()), Just("2".to_string()), Just("3".to_string()), Just("4".to_string()),
        Just("5".to_string()), Just("6".to_string()), Just("-6".to_string()), Just("0".to_string()),
        Just("-1".to_string()), Just("999999999999".to_string()), Just("1e308".to_string()),
        Just("\\001".to_string()), Just("\n".to_string()), Just("#x\n".to_string()), Just("nan".to_string()),
    ], 0..200)) {
        let mut src = String::from("#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100\nSingle\n-2\n1200 2\n");
        src.push_str(&tokens.join(" "));
        let _ = read_fig(src.as_bytes());
    }
}
