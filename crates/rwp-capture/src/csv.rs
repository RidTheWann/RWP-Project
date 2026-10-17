//! PresentMon CSV parsing and QPC-relative normalization.

use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufRead, BufReader, Read},
    path::Path,
};

use rwp_core::{CoreError, FrameSample};

use crate::{command::PresentMonTarget, error::PresentMonCsvError};

/// Normalized output of one single-process PresentMon CSV capture.
#[derive(Debug, Clone, PartialEq)]
pub struct PresentMonCapture {
    /// Normalized frame observations retained from the dominant swap chain.
    pub frames: Vec<FrameSample>,
    /// Process identifier observed throughout the capture.
    pub process_id: u32,
    /// Application name reported by PresentMon, or <unknown> when unavailable.
    pub application: String,
    /// Dominant swap-chain address, when PresentMon supplied usable values.
    pub dominant_swap_chain_address: Option<u64>,
    /// Number of source rows discarded because they were outside the dominant swap chain.
    pub discarded_frame_count: usize,
    /// Number of source rows dropped because their QPC timestamp moved
    /// backwards. Real PresentMon output occasionally emits isolated rows out
    /// of order (multi-threaded emission); dropping the row keeps the
    /// timestamp chain monotonic while the count preserves provenance.
    pub dropped_nonmonotonic_rows: usize,
    /// Frame-type distribution retained from the dominant swap chain.
    pub frame_type_counts: Vec<(String, usize)>,
}

/// Builds the dropped-rows warning message for a parsed capture, so live
/// sessions, CLI import, and GUI surfaces share one stable wording.
/// Returns `None` when nothing was dropped. Pure across platforms.
pub fn dropped_rows_notice(dropped_nonmonotonic_rows: usize) -> Option<String> {
    (dropped_nonmonotonic_rows > 0).then(|| {
        format!(
            "dropped {dropped_nonmonotonic_rows} rows with backwards QPC timestamps; source CSV retained, counts recorded here"
        )
    })
}

/// Parsing strictness for PresentMon CSV input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseMode {
    /// Reject every malformed row.
    Strict,
    /// Ignore only a malformed final row that was not newline-terminated.
    TolerateTruncatedTail,
}

/// Maximum PresentMon CSV file size accepted by the parser. This bounds attacker-controlled allocation and I/O.
const MAX_CSV_BYTES: u64 = 512 * 1024 * 1024;
/// Maximum frame rows accepted from one CSV. This bounds the in-memory normalized capture.
const MAX_FRAME_ROWS: usize = 10_000_000;
/// Maximum bytes accepted for one CSV line, excluding its terminator. A single
/// unterminated line longer than this is rejected instead of being read whole.
const MAX_CSV_LINE_BYTES: usize = 1024 * 1024;

/// Parser for the PresentMon 2.x metrics CSV schema emitted by the M2 runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresentMonCsvParser {
    qpc_frequency_hz: u64,
    qpc_origin: Option<u64>,
}

impl PresentMonCsvParser {
    /// Creates a parser using the caller-provided Windows QPC frequency.
    pub fn new(qpc_frequency_hz: u64) -> Result<Self, PresentMonCsvError> {
        if qpc_frequency_hz == 0 {
            return Err(PresentMonCsvError::InvalidQpcFrequency);
        }
        Ok(Self {
            qpc_frequency_hz,
            qpc_origin: None,
        })
    }

    /// Creates a parser using a shared absolute QPC origin.
    pub fn new_with_qpc_origin(
        qpc_frequency_hz: u64,
        qpc_origin: u64,
    ) -> Result<Self, PresentMonCsvError> {
        if qpc_frequency_hz == 0 {
            return Err(PresentMonCsvError::InvalidQpcFrequency);
        }
        Ok(Self {
            qpc_frequency_hz,
            qpc_origin: Some(qpc_origin),
        })
    }

    /// Parses a PresentMon CSV file into normalized frame samples.
    pub fn parse_path(
        &self,
        path: &Path,
        target: &PresentMonTarget,
    ) -> Result<PresentMonCapture, PresentMonCsvError> {
        self.parse_path_with_mode(path, target, ParseMode::Strict)
    }

    /// Parses a PresentMon CSV file using an explicit interruption policy.
    pub fn parse_path_with_mode(
        &self,
        path: &Path,
        target: &PresentMonTarget,
        mode: ParseMode,
    ) -> Result<PresentMonCapture, PresentMonCsvError> {
        let metadata = std::fs::metadata(path).map_err(PresentMonCsvError::Io)?;
        validate_csv_size(metadata.len())?;
        let file = File::open(path).map_err(PresentMonCsvError::Io)?;
        self.parse_reader_with_mode(BufReader::new(file), target, mode)
    }

    /// Parses PresentMon CSV data from any UTF-8 reader in strict mode.
    pub fn parse_reader<R: Read>(
        &self,
        reader: R,
        target: &PresentMonTarget,
    ) -> Result<PresentMonCapture, PresentMonCsvError> {
        self.parse_reader_with_mode(reader, target, ParseMode::Strict)
    }

    /// Parses PresentMon CSV data using an explicit interruption policy.
    pub fn parse_reader_with_mode<R: Read>(
        &self,
        reader: R,
        target: &PresentMonTarget,
        mode: ParseMode,
    ) -> Result<PresentMonCapture, PresentMonCsvError> {
        self.parse_reader_with_limits(reader, target, mode, MAX_CSV_BYTES, MAX_CSV_LINE_BYTES)
    }

    fn parse_reader_with_limits<R: Read>(
        &self,
        reader: R,
        target: &PresentMonTarget,
        mode: ParseMode,
        max_bytes: u64,
        max_line_bytes: usize,
    ) -> Result<PresentMonCapture, PresentMonCsvError> {
        let mut reader = BufReader::new(reader.take(max_bytes.saturating_add(1)));
        let mut total_bytes = 0_u64;
        let header_raw = match read_bounded_line(&mut reader, max_line_bytes, 0)? {
            Some(raw) => raw,
            None => return Err(PresentMonCsvError::EmptyCapture),
        };
        total_bytes = total_bytes.saturating_add(header_raw.len() as u64);
        if total_bytes > max_bytes {
            return Err(PresentMonCsvError::InputTooLarge {
                maximum_bytes: max_bytes,
                actual_bytes: total_bytes,
            });
        }
        let header_text = decode_strict(&header_raw).map_err(|error| {
            PresentMonCsvError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, error))
        })?;
        let headers = parse_csv_line(header_text.trim_end_matches(['\r', '\n']), 0)?;
        let headers = headers
            .into_iter()
            .map(|mut header| {
                if header.starts_with('\u{feff}') {
                    header.remove(0);
                }
                header.trim().to_owned()
            })
            .collect::<Vec<_>>();

        let columns = Columns::from_headers(&headers)?;
        let mut parsed_frames = Vec::new();
        let mut first_qpc = None;
        let mut previous_qpc = None;
        let mut dropped_nonmonotonic_rows = 0_usize;
        let mut process_id = None;
        let mut application = None;
        let mut row = 1_usize;

        while let Some(raw) = read_bounded_line(&mut reader, max_line_bytes, row)? {
            total_bytes = total_bytes.saturating_add(raw.len() as u64);
            if total_bytes > max_bytes {
                return Err(PresentMonCsvError::InputTooLarge {
                    maximum_bytes: max_bytes,
                    actual_bytes: total_bytes,
                });
            }
            let terminated = raw.ends_with(b"\n");
            let line = match String::from_utf8(strip_line_terminator(&raw).to_vec()) {
                Ok(line) => line,
                Err(_) if mode == ParseMode::TolerateTruncatedTail && !terminated => break,
                Err(error) => {
                    return Err(PresentMonCsvError::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        error,
                    )));
                }
            };
            if line.trim().is_empty() {
                row = row.saturating_add(1);
                continue;
            }

            let fields = match parse_csv_line(&line, row) {
                Ok(fields) => fields,
                Err(_error) if mode == ParseMode::TolerateTruncatedTail && !terminated => {
                    break;
                }
                Err(error) => return Err(error),
            };
            if fields.len() != headers.len() {
                if mode == ParseMode::TolerateTruncatedTail && !terminated {
                    break;
                }
                return Err(PresentMonCsvError::MalformedCsv {
                    row,
                    reason: format!(
                        "expected {} fields from header, got {}",
                        headers.len(),
                        fields.len()
                    ),
                });
            }

            let observed_pid = parse_u32(
                &fields[columns.process_id],
                row,
                &headers[columns.process_id],
            )?;
            if let Some(expected_pid) = process_id {
                if observed_pid != expected_pid {
                    return Err(PresentMonCsvError::ProcessIdChanged {
                        row,
                        expected_pid,
                        observed_pid,
                    });
                }
            } else {
                process_id = Some(observed_pid);
            }

            match target {
                PresentMonTarget::ProcessId(expected_pid) if observed_pid != *expected_pid => {
                    return Err(PresentMonCsvError::TargetMismatch {
                        row,
                        reason: format!("expected PID {expected_pid}, got {observed_pid}"),
                    });
                }
                PresentMonTarget::ProcessName(expected_name) => {
                    let observed_name = fields[columns.application].trim();
                    if !observed_name.is_empty()
                        && !observed_name.eq_ignore_ascii_case("<unknown>")
                        && !process_name_matches(expected_name, observed_name)
                    {
                        return Err(PresentMonCsvError::TargetMismatch {
                            row,
                            reason: format!(
                                "expected process name {expected_name}, got {observed_name}"
                            ),
                        });
                    }
                    if application.is_none()
                        && !observed_name.is_empty()
                        && !observed_name.eq_ignore_ascii_case("<unknown>")
                    {
                        application = Some(observed_name.to_owned());
                    }
                }
                PresentMonTarget::ProcessId(_) => {}
            }

            let qpc = parse_u64(
                &fields[columns.cpu_start_qpc],
                row,
                &headers[columns.cpu_start_qpc],
            )?;
            if let Some(previous) = previous_qpc
                && qpc < previous
            {
                // Isolated out-of-order rows occur in real PresentMon output
                // (multi-threaded emission); drop the row and count it instead
                // of failing the whole capture. The monotonic chain keeps the
                // previous high-water mark so later rows still validate.
                dropped_nonmonotonic_rows = dropped_nonmonotonic_rows.saturating_add(1);
                row = row.saturating_add(1);
                continue;
            }
            previous_qpc = Some(qpc);
            let base = match self.qpc_origin {
                Some(origin) => {
                    if qpc < origin {
                        return Err(PresentMonCsvError::QpcBeforeOrigin {
                            row,
                            origin,
                            current: qpc,
                        });
                    }
                    origin
                }
                None => *first_qpc.get_or_insert(qpc),
            };
            let delta_qpc = qpc - base;
            let timestamp_ns = (u128::from(delta_qpc) * 1_000_000_000_u128
                / u128::from(self.qpc_frequency_hz))
            .try_into()
            .map_err(|_| PresentMonCsvError::TimestampOverflow { row })?;

            let frame_time_ms = parse_f64(
                &fields[columns.frame_time],
                row,
                &headers[columns.frame_time],
            )?;
            let mut frame = FrameSample::new(timestamp_ns, frame_time_ms).map_err(|error| {
                invalid_core_metric(
                    row,
                    &headers[columns.frame_time],
                    &fields[columns.frame_time],
                    error,
                )
            })?;

            frame = frame
                .with_telemetry(
                    parse_optional_f64(&fields[columns.cpu_busy], row, &headers[columns.cpu_busy])?,
                    parse_optional_f64(&fields[columns.cpu_wait], row, &headers[columns.cpu_wait])?,
                    parse_optional_f64(&fields[columns.gpu_time], row, &headers[columns.gpu_time])?,
                    parse_optional_f64(&fields[columns.gpu_busy], row, &headers[columns.gpu_busy])?,
                    parse_optional_f64(&fields[columns.gpu_wait], row, &headers[columns.gpu_wait])?,
                    columns
                        .display_latency
                        .map(|column| parse_optional_f64(&fields[column], row, &headers[column]))
                        .transpose()?
                        .flatten(),
                )
                .map_err(|error| {
                    invalid_core_metric(row, "frame telemetry", "<multiple>", error)
                })?;

            let swap_chain_address = columns
                .swap_chain_address
                .map(|column| parse_optional_u64(&fields[column], row, &headers[column]))
                .transpose()?
                .flatten();
            let frame_type = columns
                .frame_type
                .and_then(|column| parse_optional_text(&fields[column]));
            validate_frame_row_limit(parsed_frames.len())?;
            parsed_frames.push((frame, swap_chain_address, frame_type));
            row = row.saturating_add(1);
        }

        if parsed_frames.is_empty() {
            return Err(PresentMonCsvError::EmptyCapture);
        }

        let mut swap_chain_counts = BTreeMap::new();
        for (_, address, _) in &parsed_frames {
            if let Some(address) = address {
                *swap_chain_counts.entry(*address).or_insert(0_usize) += 1;
            }
        }
        let dominant_swap_chain_address = dominant_swap_chain(&swap_chain_counts);
        let source_frame_count = parsed_frames.len();
        let mut frames = Vec::with_capacity(source_frame_count);
        let mut frame_type_counts = BTreeMap::new();
        for (frame, address, frame_type) in parsed_frames {
            let keep = match dominant_swap_chain_address {
                None => true,
                Some(dominant) => address.is_some_and(|address| address == dominant),
            };
            if keep {
                frames.push(frame);
                if let Some(frame_type) = frame_type {
                    *frame_type_counts.entry(frame_type).or_insert(0_usize) += 1;
                }
            }
        }
        if frames.is_empty() {
            return Err(PresentMonCsvError::EmptyCapture);
        }

        let process_id = process_id.ok_or(PresentMonCsvError::EmptyCapture)?;
        let application = application.unwrap_or_else(|| "<unknown>".to_owned());

        let discarded_frame_count = source_frame_count.saturating_sub(frames.len());
        Ok(PresentMonCapture {
            frames,
            process_id,
            application,
            dominant_swap_chain_address,
            discarded_frame_count,
            dropped_nonmonotonic_rows,
            frame_type_counts: frame_type_counts.into_iter().collect(),
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct Columns {
    application: usize,
    process_id: usize,
    cpu_start_qpc: usize,
    frame_time: usize,
    cpu_busy: usize,
    cpu_wait: usize,
    gpu_time: usize,
    gpu_busy: usize,
    gpu_wait: usize,
    display_latency: Option<usize>,
    swap_chain_address: Option<usize>,
    frame_type: Option<usize>,
}

impl Columns {
    fn from_headers(headers: &[String]) -> Result<Self, PresentMonCsvError> {
        Ok(Self {
            application: required(headers, "Application")?,
            process_id: required(headers, "ProcessID")?,
            cpu_start_qpc: required(headers, "CPUStartQPC")?,
            frame_time: required(headers, "FrameTime")?,
            cpu_busy: required(headers, "CPUBusy")?,
            cpu_wait: required(headers, "CPUWait")?,
            gpu_time: required(headers, "GPUTime")?,
            gpu_busy: required(headers, "GPUBusy")?,
            gpu_wait: required(headers, "GPUWait")?,
            display_latency: find(headers, "DisplayLatency"),
            swap_chain_address: find(headers, "SwapChainAddress"),
            frame_type: find(headers, "FrameType"),
        })
    }
}

fn validate_csv_size(actual_bytes: u64) -> Result<(), PresentMonCsvError> {
    if actual_bytes > MAX_CSV_BYTES {
        return Err(PresentMonCsvError::InputTooLarge {
            maximum_bytes: MAX_CSV_BYTES,
            actual_bytes,
        });
    }
    Ok(())
}

fn validate_frame_row_limit(current_rows: usize) -> Result<(), PresentMonCsvError> {
    if current_rows >= MAX_FRAME_ROWS {
        return Err(PresentMonCsvError::TooManyRows {
            maximum_rows: MAX_FRAME_ROWS,
        });
    }
    Ok(())
}

fn required(headers: &[String], name: &'static str) -> Result<usize, PresentMonCsvError> {
    find(headers, name).ok_or(PresentMonCsvError::MissingColumn(name))
}

fn find(headers: &[String], name: &str) -> Option<usize> {
    headers
        .iter()
        .position(|header| header.eq_ignore_ascii_case(name))
}

fn parse_u32(value: &str, row: usize, column: &str) -> Result<u32, PresentMonCsvError> {
    value
        .trim()
        .parse::<u32>()
        .map_err(|error| PresentMonCsvError::InvalidMetric {
            row,
            column: column.to_owned(),
            value: value.to_owned(),
            reason: error.to_string(),
        })
}

fn parse_u64(value: &str, row: usize, column: &str) -> Result<u64, PresentMonCsvError> {
    value
        .trim()
        .parse::<u64>()
        .map_err(|error| PresentMonCsvError::InvalidMetric {
            row,
            column: column.to_owned(),
            value: value.to_owned(),
            reason: error.to_string(),
        })
}

fn dominant_swap_chain(counts: &BTreeMap<u64, usize>) -> Option<u64> {
    let mut dominant = None;
    for (&address, &count) in counts {
        match dominant {
            None => dominant = Some((address, count)),
            Some((best_address, best_count))
                if count > best_count || (count == best_count && address < best_address) =>
            {
                dominant = Some((address, count));
            }
            Some(_) => {}
        }
    }
    dominant.map(|(address, _)| address)
}

fn parse_optional_u64(
    value: &str,
    row: usize,
    column: &str,
) -> Result<Option<u64>, PresentMonCsvError> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("na")
        || trimmed.eq_ignore_ascii_case("n/a")
    {
        return Ok(None);
    }
    let parsed = if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        u64::from_str_radix(hex, 16)
    } else {
        trimmed.parse::<u64>()
    };
    parsed
        .map(Some)
        .map_err(|error| PresentMonCsvError::InvalidMetric {
            row,
            column: column.to_owned(),
            value: value.to_owned(),
            reason: error.to_string(),
        })
}

fn parse_optional_text(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("na")
        || trimmed.eq_ignore_ascii_case("n/a")
    {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

fn parse_f64(value: &str, row: usize, column: &str) -> Result<f64, PresentMonCsvError> {
    value
        .trim()
        .parse::<f64>()
        .map_err(|error| PresentMonCsvError::InvalidMetric {
            row,
            column: column.to_owned(),
            value: value.to_owned(),
            reason: error.to_string(),
        })
}

fn parse_optional_f64(
    value: &str,
    row: usize,
    column: &str,
) -> Result<Option<f64>, PresentMonCsvError> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("na")
        || trimmed.eq_ignore_ascii_case("n/a")
    {
        return Ok(None);
    }
    parse_f64(trimmed, row, column).map(Some)
}

fn invalid_core_metric(
    row: usize,
    column: &str,
    value: &str,
    error: CoreError,
) -> PresentMonCsvError {
    PresentMonCsvError::InvalidMetric {
        row,
        column: column.to_owned(),
        value: value.to_owned(),
        reason: error.to_string(),
    }
}

/// Reads one newline-terminated record bounded to `max_line_bytes` of content.
///
/// Returns `None` at end of input. A record whose content exceeds the limit
/// without a terminator in range is rejected instead of being read whole, so
/// a single unterminated line can never stage unbounded memory.
fn read_bounded_line<R: BufRead>(
    reader: &mut R,
    max_line_bytes: usize,
    row: usize,
) -> Result<Option<Vec<u8>>, PresentMonCsvError> {
    let mut raw = Vec::new();
    let limit = (max_line_bytes as u64).saturating_add(1);
    reader
        .by_ref()
        .take(limit)
        .read_until(b'\n', &mut raw)
        .map_err(PresentMonCsvError::Io)?;
    if raw.is_empty() {
        return Ok(None);
    }
    let content_len = raw.len() - usize::from(raw.ends_with(b"\n"));
    if content_len > max_line_bytes {
        return Err(PresentMonCsvError::MalformedCsv {
            row,
            reason: String::from("line exceeds the configured limit"),
        });
    }
    Ok(Some(raw))
}

/// Removes one trailing newline and its optional carriage return.
fn strip_line_terminator(raw: &[u8]) -> &[u8] {
    let mut end = raw.len();
    if end > 0 && raw[end - 1] == b'\n' {
        end -= 1;
    }
    if end > 0 && raw[end - 1] == b'\r' {
        end -= 1;
    }
    &raw[..end]
}

/// Decodes one record strictly; invalid UTF-8 is the caller's decision.
fn decode_strict(raw: &[u8]) -> Result<String, std::string::FromUtf8Error> {
    String::from_utf8(strip_line_terminator(raw).to_vec())
}

fn process_name_matches(expected: &str, observed: &str) -> bool {
    let expected = expected.trim();
    let observed = observed.trim();
    if expected.eq_ignore_ascii_case(observed) {
        return true;
    }
    let expected = expected.strip_suffix(".exe").unwrap_or(expected);
    let observed = observed.strip_suffix(".exe").unwrap_or(observed);
    expected.eq_ignore_ascii_case(observed)
}

fn parse_csv_line(line: &str, row: usize) -> Result<Vec<String>, PresentMonCsvError> {
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut after_quote = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if in_quotes {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    let _escaped_quote = chars.next();
                } else {
                    in_quotes = false;
                    after_quote = true;
                }
            } else {
                field.push(ch);
            }
            continue;
        }

        if after_quote {
            match ch {
                ',' => {
                    fields.push(field);
                    field = String::new();
                    after_quote = false;
                }
                ch if ch.is_whitespace() => {}
                _ => {
                    return Err(PresentMonCsvError::MalformedCsv {
                        row,
                        reason: "characters found after a closing quote".to_owned(),
                    });
                }
            }
            continue;
        }

        match ch {
            '"' if field.is_empty() => in_quotes = true,
            ',' => {
                fields.push(field);
                field = String::new();
            }
            _ => field.push(ch),
        }
    }

    if in_quotes {
        return Err(PresentMonCsvError::MalformedCsv {
            row,
            reason: "unterminated quoted field".to_owned(),
        });
    }

    fields.push(field);
    Ok(fields)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use proptest::prelude::*;

    use super::{ParseMode, PresentMonCsvParser};

    use crate::command::PresentMonTarget;

    proptest! {
        #[test]
        fn arbitrary_csv_bytes_do_not_panic(bytes in prop::collection::vec(any::<u8>(), 0..4096)) {
            let parser = PresentMonCsvParser {
                qpc_frequency_hz: 1_000_000,
                qpc_origin: None,
            };
            let _ = parser.parse_reader(
                Cursor::new(bytes),
                &PresentMonTarget::ProcessId(1),
            );
        }

        #[test]
        fn well_formed_presentmon_rows_parse_deterministically(
            frame_times in prop::collection::vec(0.1_f64..100.0_f64, 1..32)
        ) {
            let mut csv = String::from(
                "Application,ProcessID,CPUStartQPC,FrameTime,CPUBusy,CPUWait,GPUTime,GPUBusy,GPUWait,DisplayLatency\n",
            );
            let mut rendered_times = Vec::with_capacity(frame_times.len());
            for (index, frame_time) in frame_times.iter().enumerate() {
                let qpc = 10_000_u64 + (index as u64 * 80);
                let rendered = format!("{frame_time:.6}");
                csv.push_str(&format!(
                    "Game.exe,42,{qpc},{rendered},5.0,1.0,7.0,6.0,1.0,9.0\n"
                ));
                rendered_times.push(rendered);
            }
            let parser = PresentMonCsvParser::new(10_000)
                .map_err(|error| TestCaseError::fail(error.to_string()))?;
            let capture = parser
                .parse_reader(csv.as_bytes(), &PresentMonTarget::ProcessId(42))
                .map_err(|error| TestCaseError::fail(error.to_string()))?;
            prop_assert_eq!(capture.frames.len(), frame_times.len());
            prop_assert_eq!(capture.process_id, 42);
            prop_assert_eq!(capture.discarded_frame_count, 0);
            for (frame, rendered) in capture.frames.iter().zip(rendered_times.iter()) {
                let expected = rendered
                    .parse::<f64>()
                    .map_err(|error| TestCaseError::fail(error.to_string()))?;
                prop_assert_eq!(frame.frame_time_ms, expected);
            }
        }
    }

    const SEED_CSV: &str =
        include_str!("../../../fuzz/corpus/csv_strict/minimal-presentmon-v2.csv");

    fn mutate_seed_bytes(seed: &[u8], cases: usize, mut apply: impl FnMut(&[u8])) {
        let mut state = 0x9E37_79B9_7F4A_7C15_u64;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 56) as u8
        };
        for _ in 0..cases {
            let mut input = seed.to_vec();
            let operations = usize::from(next() % 4);
            for _ in 0..=operations {
                let index = usize::from(next()) % (input.len() + 1);
                match next() % 3 {
                    0 => {
                        if index < input.len() {
                            input[index] ^= next() | 1;
                        }
                    }
                    1 => {
                        if index < input.len() {
                            input.remove(index);
                        }
                    }
                    _ => {
                        if input.len() < 4096 {
                            input.insert(index.min(input.len()), next());
                        }
                    }
                }
            }
            apply(&input);
        }
    }

    #[test]
    fn seeded_corpus_mutations_do_not_panic_in_either_mode()
    -> Result<(), Box<dyn std::error::Error>> {
        let parser = PresentMonCsvParser::new(1_000_000)?;
        let name_target = PresentMonTarget::ProcessName(String::from("Game.exe"));
        let id_target = PresentMonTarget::ProcessId(42);
        let _ = parser.parse_reader_with_mode(
            SEED_CSV.as_bytes(),
            &name_target,
            ParseMode::TolerateTruncatedTail,
        );
        mutate_seed_bytes(SEED_CSV.as_bytes(), 256, |input| {
            let _ = parser.parse_reader_with_mode(input, &name_target, ParseMode::Strict);
            let _ =
                parser.parse_reader_with_mode(input, &id_target, ParseMode::TolerateTruncatedTail);
        });
        Ok(())
    }

    const HEADER: &str = "Application,ProcessID,CPUStartQPC,FrameTime,CPUBusy,CPUWait,GPUTime,GPUBusy,GPUWait,DisplayLatency";
    const HEADER_V2: &str = "Application,ProcessID,CPUStartQPC,FrameTime,CPUBusy,CPUWait,GPUTime,GPUBusy,GPUWait,DisplayLatency,SwapChainAddress,FrameType";

    #[test]
    fn normalizes_qpc_to_capture_relative_nanoseconds() -> Result<(), Box<dyn std::error::Error>> {
        let csv = [
            HEADER,
            "Game.exe,42,1100,8.0,5.0,1.0,7.0,6.0,1.0,9.0",
            "Game.exe,42,1180,8.0,5.0,1.0,7.0,6.0,1.0,9.0",
        ]
        .join("\n");
        let parser = PresentMonCsvParser::new_with_qpc_origin(10_000, 1000)?;
        let capture = parser.parse_reader(csv.as_bytes(), &PresentMonTarget::ProcessId(42))?;
        assert_eq!(capture.frames[0].timestamp_ns, 10_000_000);
        assert_eq!(capture.frames[1].timestamp_ns, 18_000_000);
        Ok(())
    }

    #[test]
    fn keeps_dominant_swap_chain_and_counts_frame_types() -> Result<(), Box<dyn std::error::Error>>
    {
        let csv = [
            HEADER_V2,
            "Game.exe,42,1000,8.0,5.0,1.0,7.0,6.0,1.0,9.0,0x200,Application",
            "Game.exe,42,1100,8.0,5.0,1.0,7.0,6.0,1.0,9.0,0x100,Application",
            "Game.exe,42,1200,8.0,5.0,1.0,7.0,6.0,1.0,9.0,0x100,Repeated",
        ]
        .join("\n");
        let parser = PresentMonCsvParser::new(1_000_000)?;
        let capture = parser.parse_reader(csv.as_bytes(), &PresentMonTarget::ProcessId(42))?;
        assert_eq!(capture.dominant_swap_chain_address, Some(0x100));
        assert_eq!(capture.discarded_frame_count, 1);
        assert_eq!(capture.frames.len(), 2);
        assert_eq!(
            capture.frame_type_counts,
            vec![
                (String::from("Application"), 1),
                (String::from("Repeated"), 1),
            ]
        );
        Ok(())
    }

    #[test]
    fn missing_swap_chain_values_remain_unfiltered() -> Result<(), Box<dyn std::error::Error>> {
        let csv = format!("{HEADER_V2}\nGame.exe,42,1000,8.0,5.0,1.0,7.0,6.0,1.0,9.0,NA,NotSet\n");
        let parser = PresentMonCsvParser::new(1_000_000)?;
        let capture = parser.parse_reader(csv.as_bytes(), &PresentMonTarget::ProcessId(42))?;
        assert_eq!(capture.dominant_swap_chain_address, None);
        assert_eq!(capture.discarded_frame_count, 0);
        assert_eq!(capture.frame_type_counts, vec![(String::from("NotSet"), 1)]);
        Ok(())
    }

    #[test]
    fn rejects_qpc_before_explicit_origin() -> Result<(), Box<dyn std::error::Error>> {
        let csv = [HEADER, "Game.exe,42,900,8.0,5.0,1.0,7.0,6.0,1.0,9.0"].join("\n");
        let parser = PresentMonCsvParser::new_with_qpc_origin(1_000_000, 1000)?;
        assert!(matches!(
            parser.parse_reader(csv.as_bytes(), &PresentMonTarget::ProcessId(42)),
            Err(crate::error::PresentMonCsvError::QpcBeforeOrigin { .. })
        ));
        Ok(())
    }

    #[test]
    fn accepts_na_as_absent_optional_telemetry() -> Result<(), Box<dyn std::error::Error>> {
        let csv = format!("{HEADER}\nGame.exe,42,1000,8.0,NA,1.0,7.0,NA,1.0,NA\n");
        let parser = PresentMonCsvParser::new(1_000_000)?;
        let capture = parser.parse_reader(csv.as_bytes(), &PresentMonTarget::ProcessId(42))?;
        let frame = &capture.frames[0];
        assert_eq!(frame.cpu_busy_ms, None);
        assert_eq!(frame.gpu_busy_ms, None);
        assert_eq!(frame.display_latency_ms, None);
        Ok(())
    }

    #[test]
    fn rejects_wrong_process_identity() -> Result<(), Box<dyn std::error::Error>> {
        let csv = format!("{HEADER}\nOther.exe,42,1000,8.0,5.0,1.0,7.0,6.0,1.0,9.0\n");
        let parser = PresentMonCsvParser::new(1_000_000)?;
        let result = parser.parse_reader(
            csv.as_bytes(),
            &PresentMonTarget::ProcessName("Game.exe".to_owned()),
        );
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn tolerates_truncated_final_record() -> Result<(), Box<dyn std::error::Error>> {
        let csv = format!(
            "{HEADER}\nGame.exe,42,1000,8.0,5.0,1.0,7.0,6.0,1.0,9.0\nGame.exe,42,1100,8.0,5.0,1.0"
        );
        let parser = PresentMonCsvParser::new(1_000_000)?;
        assert!(
            parser
                .parse_reader(csv.as_bytes(), &PresentMonTarget::ProcessId(42))
                .is_err()
        );
        let capture = parser.parse_reader_with_mode(
            csv.as_bytes(),
            &PresentMonTarget::ProcessId(42),
            ParseMode::TolerateTruncatedTail,
        )?;
        assert_eq!(capture.frames.len(), 1);
        Ok(())
    }

    #[test]
    fn drops_backwards_qpc_row_and_counts_it() -> Result<(), Box<dyn std::error::Error>> {
        // Real PresentMon 2.6.0 output observed in the field moves QPC
        // backwards on isolated rows (multi-threaded emission order); the
        // row is dropped and counted instead of failing the session.
        let csv = format!(
            "{HEADER}\nGame.exe,42,1000,8.0,5.0,1.0,7.0,6.0,1.0,9.0\nGame.exe,42,900,8.0,5.0,1.0,7.0,6.0,1.0,9.0\nGame.exe,42,1100,8.0,5.0,1.0,7.0,6.0,1.0,9.0\n"
        );
        let parser = PresentMonCsvParser::new(1_000_000)?;
        let capture = parser.parse_reader(csv.as_bytes(), &PresentMonTarget::ProcessId(42))?;
        assert_eq!(capture.frames.len(), 2);
        assert_eq!(capture.dropped_nonmonotonic_rows, 1);
        assert_eq!(capture.discarded_frame_count, 0);
        Ok(())
    }

    const REAL_CAPTURE_V2_ANONYMIZED: &str =
        include_str!("../tests/fixtures/real-capture-v2-anonymized.csv");

    #[test]
    fn real_capture_window_drops_backwards_rows_and_keeps_monotonic_frames()
    -> Result<(), Box<dyn std::error::Error>> {
        // Anonymized 320-row window of the 2026-10-17 live dwm.exe capture
        // (PresentMon 2.6.0, QPC 10 MHz; see docs/acceptance/2026-10-17.md):
        // 32 backwards-QPC rows dropped, 33 rows outside the dominant swap
        // chain discarded, 255 frames kept with non-decreasing timestamps.
        let parser = PresentMonCsvParser::new(10_000_000)?;
        let capture = parser.parse_reader(
            REAL_CAPTURE_V2_ANONYMIZED.as_bytes(),
            &PresentMonTarget::ProcessId(8852),
        )?;
        assert_eq!(capture.process_id, 8852);
        assert_eq!(capture.frames.len(), 255);
        assert_eq!(capture.dropped_nonmonotonic_rows, 32);
        assert_eq!(capture.discarded_frame_count, 33);
        assert!(
            capture
                .frames
                .windows(2)
                .all(|pair| pair[0].timestamp_ns <= pair[1].timestamp_ns)
        );
        Ok(())
    }

    #[test]
    fn resource_limits_accept_the_boundary_and_reject_overflow() {
        assert!(super::validate_csv_size(super::MAX_CSV_BYTES).is_ok());
        assert!(matches!(
            super::validate_csv_size(super::MAX_CSV_BYTES + 1),
            Err(crate::error::PresentMonCsvError::InputTooLarge {
                maximum_bytes,
                actual_bytes,
            }) if maximum_bytes == super::MAX_CSV_BYTES
                && actual_bytes == super::MAX_CSV_BYTES + 1
        ));

        assert!(super::validate_frame_row_limit(super::MAX_FRAME_ROWS - 1).is_ok());
        assert!(matches!(
            super::validate_frame_row_limit(super::MAX_FRAME_ROWS),
            Err(crate::error::PresentMonCsvError::TooManyRows { maximum_rows })
                if maximum_rows == super::MAX_FRAME_ROWS
        ));
    }

    #[test]
    fn parses_quoted_fields_and_escaped_quotes() -> Result<(), Box<dyn std::error::Error>> {
        let fields = super::parse_csv_line(r#""Game, Inc.","42","1000","8.0""#, 1)?;
        assert_eq!(fields[0], "Game, Inc.");
        assert_eq!(fields[1], "42");
        assert_eq!(fields[2], "1000");
        assert_eq!(fields[3], "8.0");
        Ok(())
    }

    #[test]
    fn rejects_zero_qpc_frequency() {
        assert!(matches!(
            super::PresentMonCsvParser::new(0),
            Err(crate::error::PresentMonCsvError::InvalidQpcFrequency)
        ));
        assert!(matches!(
            super::PresentMonCsvParser::new_with_qpc_origin(0, 1000),
            Err(crate::error::PresentMonCsvError::InvalidQpcFrequency)
        ));
    }

    #[test]
    fn rejects_header_only_capture() -> Result<(), Box<dyn std::error::Error>> {
        let parser = super::PresentMonCsvParser::new(1_000_000)?;
        assert!(matches!(
            parser.parse_reader(HEADER.as_bytes(), &PresentMonTarget::ProcessId(42)),
            Err(crate::error::PresentMonCsvError::EmptyCapture)
        ));
        Ok(())
    }

    #[test]
    fn strips_bom_from_headers() -> Result<(), Box<dyn std::error::Error>> {
        let csv = format!("\u{feff}{HEADER}\nGame.exe,42,1000,8.0,5.0,1.0,7.0,6.0,1.0,9.0\n");
        let parser = super::PresentMonCsvParser::new(1_000_000)?;
        let capture = parser.parse_reader(csv.as_bytes(), &PresentMonTarget::ProcessId(42))?;
        assert_eq!(capture.frames.len(), 1);
        Ok(())
    }

    #[test]
    fn skips_blank_lines_between_rows() -> Result<(), Box<dyn std::error::Error>> {
        let csv = format!(
            "{HEADER}\n\nGame.exe,42,1000,8.0,5.0,1.0,7.0,6.0,1.0,9.0\n   \nGame.exe,42,1100,8.0,5.0,1.0,7.0,6.0,1.0,9.0\n"
        );
        let parser = super::PresentMonCsvParser::new(1_000_000)?;
        let capture = parser.parse_reader(csv.as_bytes(), &PresentMonTarget::ProcessId(42))?;
        assert_eq!(capture.frames.len(), 2);
        Ok(())
    }

    #[test]
    fn rejects_field_count_mismatch() -> Result<(), Box<dyn std::error::Error>> {
        let csv = format!("{HEADER}\nGame.exe,42,1000,8.0\n");
        let parser = super::PresentMonCsvParser::new(1_000_000)?;
        assert!(matches!(
            parser.parse_reader(csv.as_bytes(), &PresentMonTarget::ProcessId(42)),
            Err(crate::error::PresentMonCsvError::MalformedCsv { .. })
        ));
        Ok(())
    }

    #[test]
    fn rejects_changed_process_id() -> Result<(), Box<dyn std::error::Error>> {
        let csv = format!(
            "{HEADER}\nGame.exe,42,1000,8.0,5.0,1.0,7.0,6.0,1.0,9.0\nGame.exe,43,1100,8.0,5.0,1.0,7.0,6.0,1.0,9.0\n"
        );
        let parser = super::PresentMonCsvParser::new(1_000_000)?;
        assert!(matches!(
            parser.parse_reader(csv.as_bytes(), &PresentMonTarget::ProcessId(42)),
            Err(crate::error::PresentMonCsvError::ProcessIdChanged { .. })
        ));
        Ok(())
    }

    #[test]
    fn rejects_pid_target_mismatch() -> Result<(), Box<dyn std::error::Error>> {
        let csv = format!("{HEADER}\nGame.exe,42,1000,8.0,5.0,1.0,7.0,6.0,1.0,9.0\n");
        let parser = super::PresentMonCsvParser::new(1_000_000)?;
        assert!(matches!(
            parser.parse_reader(csv.as_bytes(), &PresentMonTarget::ProcessId(99)),
            Err(crate::error::PresentMonCsvError::TargetMismatch { .. })
        ));
        Ok(())
    }

    #[test]
    fn rejects_qpc_timestamp_overflow() -> Result<(), Box<dyn std::error::Error>> {
        let csv = format!(
            "{HEADER}\nGame.exe,42,0,8.0,5.0,1.0,7.0,6.0,1.0,9.0\nGame.exe,42,20000000000,8.0,5.0,1.0,7.0,6.0,1.0,9.0\n"
        );
        let parser = super::PresentMonCsvParser::new(1)?;
        assert!(matches!(
            parser.parse_reader(csv.as_bytes(), &PresentMonTarget::ProcessId(42)),
            Err(crate::error::PresentMonCsvError::TimestampOverflow { .. })
        ));
        Ok(())
    }

    #[test]
    fn rejects_nonpositive_frame_time() -> Result<(), Box<dyn std::error::Error>> {
        let csv = format!("{HEADER}\nGame.exe,42,1000,0.0,5.0,1.0,7.0,6.0,1.0,9.0\n");
        let parser = super::PresentMonCsvParser::new(1_000_000)?;
        assert!(matches!(
            parser.parse_reader(csv.as_bytes(), &PresentMonTarget::ProcessId(42)),
            Err(crate::error::PresentMonCsvError::InvalidMetric { .. })
        ));
        Ok(())
    }

    #[test]
    fn rejects_negative_telemetry() -> Result<(), Box<dyn std::error::Error>> {
        let csv = format!("{HEADER}\nGame.exe,42,1000,8.0,-1.0,1.0,7.0,6.0,1.0,9.0\n");
        let parser = super::PresentMonCsvParser::new(1_000_000)?;
        assert!(matches!(
            parser.parse_reader(csv.as_bytes(), &PresentMonTarget::ProcessId(42)),
            Err(crate::error::PresentMonCsvError::InvalidMetric { .. })
        ));
        Ok(())
    }

    #[test]
    fn rejects_unparseable_metrics() -> Result<(), Box<dyn std::error::Error>> {
        let parser = super::PresentMonCsvParser::new(1_000_000)?;
        for row in [
            "Game.exe,abc,1000,8.0,5.0,1.0,7.0,6.0,1.0,9.0",
            "Game.exe,42,1000,abc,5.0,1.0,7.0,6.0,1.0,9.0",
            "Game.exe,42,1000,8.0,abc,1.0,7.0,6.0,1.0,9.0",
        ] {
            let csv = format!("{HEADER}\n{row}\n");
            assert!(
                matches!(
                    parser.parse_reader(csv.as_bytes(), &PresentMonTarget::ProcessId(42)),
                    Err(crate::error::PresentMonCsvError::InvalidMetric { .. })
                ),
                "row parses unexpectedly: {row}"
            );
        }
        Ok(())
    }

    #[test]
    fn rejects_missing_required_column() -> Result<(), Box<dyn std::error::Error>> {
        let csv = "Application,ProcessID,CPUStartQPC,CPUBusy,CPUWait,GPUTime,GPUBusy,GPUWait,DisplayLatency\n";
        let parser = super::PresentMonCsvParser::new(1_000_000)?;
        assert!(matches!(
            parser.parse_reader(csv.as_bytes(), &PresentMonTarget::ProcessId(42)),
            Err(crate::error::PresentMonCsvError::MissingColumn("FrameTime"))
        ));
        Ok(())
    }

    #[test]
    fn rejects_invalid_optional_swap_chain() -> Result<(), Box<dyn std::error::Error>> {
        let csv =
            format!("{HEADER_V2}\nGame.exe,42,1000,8.0,5.0,1.0,7.0,6.0,1.0,9.0,xyz,Application\n");
        let parser = super::PresentMonCsvParser::new(1_000_000)?;
        assert!(matches!(
            parser.parse_reader(csv.as_bytes(), &PresentMonTarget::ProcessId(42)),
            Err(crate::error::PresentMonCsvError::InvalidMetric { .. })
        ));
        Ok(())
    }

    #[test]
    fn parses_uppercase_hex_and_decimal_swap_chains() -> Result<(), Box<dyn std::error::Error>> {
        let csv = format!(
            "{HEADER_V2}\nGame.exe,42,1000,8.0,5.0,1.0,7.0,6.0,1.0,9.0,0X200,Application\nGame.exe,42,1100,8.0,5.0,1.0,7.0,6.0,1.0,9.0,512,Application\n"
        );
        let parser = super::PresentMonCsvParser::new(1_000_000)?;
        let capture = parser.parse_reader(csv.as_bytes(), &PresentMonTarget::ProcessId(42))?;
        assert_eq!(capture.dominant_swap_chain_address, Some(0x200));
        assert_eq!(capture.discarded_frame_count, 0);
        Ok(())
    }

    #[test]
    fn breaks_swap_chain_ties_toward_smaller_address() -> Result<(), Box<dyn std::error::Error>> {
        let csv = format!(
            "{HEADER_V2}\nGame.exe,42,1000,8.0,5.0,1.0,7.0,6.0,1.0,9.0,0x200,Application\nGame.exe,42,1100,8.0,5.0,1.0,7.0,6.0,1.0,9.0,0x100,Application\n"
        );
        let parser = super::PresentMonCsvParser::new(1_000_000)?;
        let capture = parser.parse_reader(csv.as_bytes(), &PresentMonTarget::ProcessId(42))?;
        assert_eq!(capture.dominant_swap_chain_address, Some(0x100));
        assert_eq!(capture.frames.len(), 1);
        Ok(())
    }

    #[test]
    fn treats_na_frame_type_as_absent() -> Result<(), Box<dyn std::error::Error>> {
        let csv = format!("{HEADER_V2}\nGame.exe,42,1000,8.0,5.0,1.0,7.0,6.0,1.0,9.0,0x100,n/a\n");
        let parser = super::PresentMonCsvParser::new(1_000_000)?;
        let capture = parser.parse_reader(csv.as_bytes(), &PresentMonTarget::ProcessId(42))?;
        assert_eq!(capture.frames.len(), 1);
        assert!(capture.frame_type_counts.is_empty());
        Ok(())
    }

    #[test]
    fn keeps_unknown_application_without_identity_mismatch()
    -> Result<(), Box<dyn std::error::Error>> {
        let csv = format!("{HEADER}\n<unknown>,42,1000,8.0,5.0,1.0,7.0,6.0,1.0,9.0\n");
        let parser = super::PresentMonCsvParser::new(1_000_000)?;
        let capture = parser.parse_reader(
            csv.as_bytes(),
            &PresentMonTarget::ProcessName("Game.exe".to_owned()),
        )?;
        assert_eq!(capture.application, "<unknown>");
        Ok(())
    }

    #[test]
    fn rejects_malformed_quoted_rows() {
        assert!(matches!(
            super::parse_csv_line("\"unterminated,42", 7),
            Err(crate::error::PresentMonCsvError::MalformedCsv { row: 7, .. })
        ));
        assert!(matches!(
            super::parse_csv_line("\"quoted\"x,42", 7),
            Err(crate::error::PresentMonCsvError::MalformedCsv { row: 7, .. })
        ));
    }

    #[test]
    fn reader_line_bound_rejects_oversized_rows() -> Result<(), Box<dyn std::error::Error>> {
        let row = "Game.exe,42,1000,8.0,5.0,1.0,7.0,6.0,1.0,9.0";
        let csv = format!("{HEADER}\n{row}\n");
        let parser = super::PresentMonCsvParser::new(1_000_000)?;
        let fitting = parser.parse_reader_with_limits(
            csv.as_bytes(),
            &PresentMonTarget::ProcessId(42),
            super::ParseMode::Strict,
            10_000,
            HEADER.len(),
        )?;
        assert_eq!(fitting.frames.len(), 1);

        let long_row = format!("{},42,1000,8.0,5.0,1.0,7.0,6.0,1.0,9.0", "G".repeat(200));
        let long_csv = format!("{HEADER}\n{long_row}\n");
        assert!(matches!(
            parser.parse_reader_with_limits(
                long_csv.as_bytes(),
                &PresentMonTarget::ProcessId(42),
                super::ParseMode::Strict,
                10_000,
                HEADER.len(),
            ),
            Err(crate::error::PresentMonCsvError::MalformedCsv { .. })
        ));

        assert!(matches!(
            parser.parse_reader_with_limits(
                csv.as_bytes(),
                &PresentMonTarget::ProcessId(42),
                super::ParseMode::Strict,
                10_000,
                10,
            ),
            Err(crate::error::PresentMonCsvError::MalformedCsv { .. })
        ));
        Ok(())
    }

    #[test]
    fn reader_size_bound_rejects_overflow() -> Result<(), Box<dyn std::error::Error>> {
        let csv = format!("{HEADER}\nGame.exe,42,1000,8.0,5.0,1.0,7.0,6.0,1.0,9.0\n");
        let parser = super::PresentMonCsvParser::new(1_000_000)?;
        let total = csv.len() as u64;
        let fitting = parser.parse_reader_with_limits(
            csv.as_bytes(),
            &PresentMonTarget::ProcessId(42),
            super::ParseMode::Strict,
            total,
            1024 * 1024,
        )?;
        assert_eq!(fitting.frames.len(), 1);
        assert!(matches!(
            parser.parse_reader_with_limits(
                csv.as_bytes(),
                &PresentMonTarget::ProcessId(42),
                super::ParseMode::Strict,
                total - 1,
                1024 * 1024,
            ),
            Err(crate::error::PresentMonCsvError::InputTooLarge { maximum_bytes, .. })
                if maximum_bytes == total - 1
        ));
        Ok(())
    }

    #[test]
    fn truncated_tail_with_split_utf8_is_dropped_in_tolerant_mode()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut csv =
            format!("{HEADER}\nGame.exe,42,1000,8.0,5.0,1.0,7.0,6.0,1.0,9.0\n").into_bytes();
        // First two bytes of U+65E5 without its final byte: invalid UTF-8 tail.
        csv.extend_from_slice(b"Game.exe,42,1100,8.0,5.0,1.0,7.0,6.0,1.0,9.0\nGame\xe6\x97");
        let parser = super::PresentMonCsvParser::new(1_000_000)?;
        assert!(
            parser
                .parse_reader(csv.as_slice(), &PresentMonTarget::ProcessId(42))
                .is_err()
        );
        let capture = parser.parse_reader_with_mode(
            csv.as_slice(),
            &PresentMonTarget::ProcessId(42),
            super::ParseMode::TolerateTruncatedTail,
        )?;
        // The complete second row parses; only the split-UTF-8 tail is dropped.
        assert_eq!(capture.frames.len(), 2);

        let mut short =
            format!("{HEADER}\nGame.exe,42,1000,8.0,5.0,1.0,7.0,6.0,1.0,9.0\n").into_bytes();
        short.extend_from_slice("caf\u{e9}".as_bytes());
        let capture = parser.parse_reader_with_mode(
            short.as_slice(),
            &PresentMonTarget::ProcessId(42),
            super::ParseMode::TolerateTruncatedTail,
        )?;
        assert_eq!(capture.frames.len(), 1);
        Ok(())
    }
}
