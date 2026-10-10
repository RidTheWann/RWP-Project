#![no_main]

use libfuzzer_sys::fuzz_target;
use rwp_capture::{ParseMode, PresentMonCsvParser, PresentMonTarget};

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(parser) = PresentMonCsvParser::new(1_000_000) else {
        return;
    };
    let target = PresentMonTarget::ProcessName("Game.exe".to_owned());
    let _ = parser.parse_reader_with_mode(text.as_bytes(), &target, ParseMode::TolerateTruncatedTail);
});
