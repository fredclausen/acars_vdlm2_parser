use std::error::Error;
use std::fs;

use acars_vdlm2_parser::{AcarsVdlm2Message, DecodeMessage};
use serde_json::{json, Value};

/// Fixture prefixes whose every line must survive decode -> re-serialise unchanged.
const LOSSLESS_PREFIXES: &[&str] = &["imsl_", "irdm_"];

fn reserialise(line: &str) -> Result<Value, Box<dyn Error>> {
    let reserialised = line.to_string().decode_message()?.to_string()?;
    Ok(serde_json::from_str(&reserialised)?)
}

fn decode_irdm(line: &str) -> Result<acars_vdlm2_parser::irdm::IrdmMessage, Box<dyn Error>> {
    match line.to_string().decode_message()? {
        AcarsVdlm2Message::IrdmMessage(irdm) => Ok(irdm),
        other => Err(format!("expected IrdmMessage, got {other:?}").into()),
    }
}

fn find_line(path: &str, predicate: impl Fn(&Value) -> bool) -> Result<String, Box<dyn Error>> {
    let contents = fs::read_to_string(path)?;
    for line in contents.lines() {
        if let Ok(value) = serde_json::from_str::<Value>(line) {
            if predicate(&value) {
                return Ok(line.to_string());
            }
        }
    }
    Err(format!("{path}: no line matches").into())
}

#[test]
fn every_imsl_and_irdm_fixture_line_round_trips_exactly() -> Result<(), Box<dyn Error>> {
    let mut failures: Vec<String> = Vec::new();
    let mut checked_lines: usize = 0;

    for entry in glob::glob("test_files/*")? {
        let path = entry?;
        let display = path.display().to_string();
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("{display}: file name is not valid UTF-8"))?;
        if !path.is_file() || !LOSSLESS_PREFIXES.iter().any(|prefix| file_name.starts_with(prefix)) {
            continue;
        }

        let contents = fs::read_to_string(&path)?;
        for (index, line) in contents.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            checked_lines += 1;
            let location = format!("{display}:{}", index + 1);
            let original: Value = match serde_json::from_str(line) {
                Ok(value) => value,
                Err(error) => {
                    failures.push(format!("{location}: fixture is not JSON: {error}"));
                    continue;
                }
            };
            match reserialise(line) {
                Ok(round_trip) if round_trip == original => {}
                Ok(round_trip) => failures.push(format!(
                    "{location}: round trip differs\n  original:     {original}\n  reserialised: {round_trip}"
                )),
                Err(error) => failures.push(format!("{location}: {error}")),
            }
        }
    }

    assert!(checked_lines > 0, "no imsl_/irdm_ fixture lines found");
    assert!(
        failures.is_empty(),
        "{} of {checked_lines} lines did not round trip:\n{}",
        failures.len(),
        failures.join("\n")
    );
    Ok(())
}

#[test]
fn imsl_level_and_libacars_survive_round_trip() -> Result<(), Box<dyn Error>> {
    let line = find_line("test_files/imsl_docker_acars", |value| {
        value.get("level").is_some() && value.get("libacars").is_some()
    })?;
    let original: Value = serde_json::from_str(&line)?;

    let round_trip = reserialise(&line)?;
    assert_eq!(round_trip["level"], original["level"]);
    assert_eq!(round_trip["libacars"], original["libacars"]);
    assert!(round_trip["libacars"].is_object());
    Ok(())
}

#[test]
fn imsl_integer_timestamp_stays_an_integer() -> Result<(), Box<dyn Error>> {
    let line = find_line("test_files/imsl_docker_acars", |value| value["timestamp"].is_u64())?;
    let original: Value = serde_json::from_str(&line)?;

    let round_trip = reserialise(&line)?;
    assert!(round_trip["timestamp"].is_u64(), "timestamp became {}", round_trip["timestamp"]);
    assert_eq!(round_trip["timestamp"], original["timestamp"]);
    Ok(())
}

#[test]
fn irdm_acars_flight_survives_round_trip() -> Result<(), Box<dyn Error>> {
    let mut value = serde_json::from_str::<Value>(&find_line("test_files/irdm_docker", |_| true)?)?;
    value["acars"]["flight"] = json!("UA0123");
    value["acars"]["message_number"] = json!("M01A");
    let line = serde_json::to_string(&value)?;

    let irdm = decode_irdm(&line)?;
    assert_eq!(irdm.acars.extra.get("flight"), Some(&json!("UA0123")));

    let round_trip = reserialise(&line)?;
    assert_eq!(round_trip["acars"]["flight"], json!("UA0123"));
    assert_eq!(round_trip["acars"]["message_number"], json!("M01A"));
    assert_eq!(round_trip, value);
    Ok(())
}

#[test]
fn irdm_integer_freq_and_level_stay_integers() -> Result<(), Box<dyn Error>> {
    let mut value = serde_json::from_str::<Value>(&find_line("test_files/irdm_docker", |_| true)?)?;
    value["freq"] = json!(1_626_270_833_i64);
    value["level"] = json!(-42_i64);
    let line = serde_json::to_string(&value)?;

    let round_trip = reserialise(&line)?;
    assert_eq!(round_trip["freq"], json!(1_626_270_833_i64));
    assert_eq!(round_trip["level"], json!(-42_i64));
    Ok(())
}

#[test]
fn unknown_top_level_and_nested_keys_survive_round_trip() -> Result<(), Box<dyn Error>> {
    let mut imsl = serde_json::from_str::<Value>(&find_line("test_files/imsl_docker_acars", |value| {
        value["signal_unit"].is_object() && value["source"]["app"].is_object()
    })?)?;
    imsl["unknown_top"] = json!({"nested": [1, 2.5, "x"]});
    imsl["signal_unit"]["unknown_su"] = json!(7);
    imsl["source"]["unknown_source"] = json!(true);
    imsl["source"]["app"]["unknown_app"] = json!("y");

    let mut irdm = serde_json::from_str::<Value>(&find_line("test_files/irdm_docker", |_| true)?)?;
    irdm["unknown_top"] = json!(null);
    irdm["app"]["unknown_app"] = json!(1);
    irdm["source"]["unknown_source"] = json!("z");
    irdm["acars"]["unknown_acars"] = json!({"deep": 3});

    for value in [imsl, irdm] {
        let round_trip = reserialise(&serde_json::to_string(&value)?)?;
        assert_eq!(round_trip, value);
    }
    Ok(())
}
