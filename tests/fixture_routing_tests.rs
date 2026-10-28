use std::collections::BTreeSet;
use std::error::Error;
use std::fs;

use acars_vdlm2_parser::{AcarsVdlm2Message, DecodeMessage};
use serde_json::Value;

/// Lines that are intentionally truncated samples and therefore not valid JSON.
const KNOWN_INVALID_JSON_LINES: &[(&str, usize)] = &[
    ("test_files/acars_3", 29),
    ("test_files/acars_3", 48),
    ("test_files/vdlm2_3", 11),
    ("test_files/vdlm2_3", 27),
];

/// Maps a fixture file name prefix to the enum variant its lines must decode into.
const PREFIX_TO_VARIANT: &[(&str, &str)] = &[
    ("acars", "AcarsMessage"),
    ("vdlm2", "Vdlm2Message"),
    ("hfdl", "HfdlMessage"),
    ("imsl", "ImslMessage"),
    ("irdm", "IrdmMessage"),
    ("xng", "XngMessage"),
];

fn variant_name(message: &AcarsVdlm2Message) -> &'static str {
    match message {
        AcarsVdlm2Message::Vdlm2Message(_) => "Vdlm2Message",
        AcarsVdlm2Message::AcarsMessage(_) => "AcarsMessage",
        AcarsVdlm2Message::HfdlMessage(_) => "HfdlMessage",
        AcarsVdlm2Message::ImslMessage(_) => "ImslMessage",
        AcarsVdlm2Message::IrdmMessage(_) => "IrdmMessage",
        AcarsVdlm2Message::XngMessage(_) => "XngMessage",
    }
}

fn expected_variant(file_name: &str) -> Option<&'static str> {
    PREFIX_TO_VARIANT
        .iter()
        .find(|(prefix, _)| file_name.starts_with(prefix))
        .map(|(_, variant)| *variant)
}

#[test]
fn every_fixture_line_routes_to_its_variant() -> Result<(), Box<dyn Error>> {
    let mut failures: Vec<String> = Vec::new();
    let mut invalid_json: BTreeSet<(String, usize)> = BTreeSet::new();
    let mut checked_lines: usize = 0;

    for entry in glob::glob("test_files/*")? {
        let path = entry?;
        if !path.is_file() {
            continue;
        }
        let display = path.display().to_string();
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("{display}: file name is not valid UTF-8"))?;
        let Some(expected) = expected_variant(file_name) else {
            failures.push(format!("{display}: unrecognised fixture prefix"));
            continue;
        };

        let contents = fs::read_to_string(&path)?;
        for (index, line) in contents.lines().enumerate() {
            let line_number = index + 1;
            if line.trim().is_empty() {
                continue;
            }
            if serde_json::from_str::<Value>(line).is_err() {
                invalid_json.insert((display.clone(), line_number));
                continue;
            }
            checked_lines += 1;
            match line.to_string().decode_message() {
                Ok(message) => {
                    let actual = variant_name(&message);
                    if actual != expected {
                        failures.push(format!(
                            "{display}:{line_number}: expected {expected}, got {actual}"
                        ));
                    }
                    if message.get_time().is_none() {
                        failures.push(format!(
                            "{display}:{line_number}: get_time() returned None ({actual})"
                        ));
                    }
                }
                Err(error) => failures.push(format!(
                    "{display}:{line_number}: expected {expected}, decode failed: {error}"
                )),
            }
        }
    }

    let expected_invalid: BTreeSet<(String, usize)> = KNOWN_INVALID_JSON_LINES
        .iter()
        .map(|(file, line)| ((*file).to_string(), *line))
        .collect();
    if invalid_json != expected_invalid {
        failures.push(format!(
            "invalid JSON line set mismatch: expected {expected_invalid:?}, found {invalid_json:?}"
        ));
    }
    if checked_lines == 0 {
        failures.push("no fixture lines were checked".to_string());
    }

    assert!(
        failures.is_empty(),
        "{} fixture routing failure(s) across {checked_lines} checked line(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
    Ok(())
}
