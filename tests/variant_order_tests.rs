use std::error::Error;
use std::fs;

use acars_vdlm2_parser::{AcarsVdlm2Message, DecodeMessage};
use serde_json::{json, Value};

/// Fixture prefixes whose lines must never be captured by `IrdmMessage` or `ImslMessage`.
const NON_SATCOM_PREFIXES: &[&str] = &["acars", "vdlm2", "hfdl", "xng"];

fn first_line(path: &str) -> Result<String, Box<dyn Error>> {
    let contents = fs::read_to_string(path)?;
    let line = contents
        .lines()
        .next()
        .ok_or_else(|| format!("{path} is empty"))?;
    Ok(line.to_string())
}

/// An IRDM message whose `app` also satisfies `AppDetails` (`name` + `ver`) and which carries a
/// numeric `freq` is a valid `AcarsMessage` too. It must still decode as `IrdmMessage`.
#[test]
fn irdm_with_app_details_shaped_app_decodes_as_irdm() -> Result<(), Box<dyn Error>> {
    let mut value: Value = serde_json::from_str(&first_line("test_files/irdm_docker")?)?;
    value["app"] = json!({"name": "iridium-toolkit", "ver": "0.0.2", "version": "0.0.2"});
    assert!(value["freq"].is_number(), "fixture must carry a numeric freq");
    let line = serde_json::to_string(&value)?;

    // Guard that the input really is ambiguous, otherwise the ordering is not being exercised.
    assert!(serde_json::from_str::<acars_vdlm2_parser::acars::AcarsMessage>(&line).is_ok());

    let message = line.decode_message()?;
    assert!(
        matches!(message, AcarsVdlm2Message::IrdmMessage(_)),
        "expected IrdmMessage, got {message:?}"
    );
    assert!(message.get_time().is_some());
    Ok(())
}

#[test]
fn non_satcom_fixtures_are_not_captured_by_irdm_or_imsl() -> Result<(), Box<dyn Error>> {
    let mut failures: Vec<String> = Vec::new();
    let mut checked_lines: usize = 0;

    for entry in glob::glob("test_files/*")? {
        let path = entry?;
        let display = path.display().to_string();
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("{display}: file name is not valid UTF-8"))?;
        if !path.is_file() || !NON_SATCOM_PREFIXES.iter().any(|prefix| file_name.starts_with(prefix)) {
            continue;
        }

        let contents = fs::read_to_string(&path)?;
        for (index, line) in contents.lines().enumerate() {
            if line.trim().is_empty() || serde_json::from_str::<Value>(line).is_err() {
                continue;
            }
            checked_lines += 1;
            match line.decode_message() {
                Ok(AcarsVdlm2Message::IrdmMessage(_)) => {
                    failures.push(format!("{display}:{}: captured by IrdmMessage", index + 1));
                }
                Ok(AcarsVdlm2Message::ImslMessage(_)) => {
                    failures.push(format!("{display}:{}: captured by ImslMessage", index + 1));
                }
                Ok(_) | Err(_) => {}
            }
        }
    }

    assert!(checked_lines > 0, "no fixture lines were checked");
    assert!(
        failures.is_empty(),
        "{} line(s) captured by a satcom variant:\n{}",
        failures.len(),
        failures.join("\n")
    );
    Ok(())
}
