use std::error::Error;
use std::fs;

use acars_vdlm2_parser::imsl::{ImslFrequency, ImslMessage};
use acars_vdlm2_parser::{AcarsVdlm2Message, DecodeMessage};
use serde_json::{json, Value};

fn first_line_value(path: &str) -> Result<Value, Box<dyn Error>> {
    let contents = fs::read_to_string(path)?;
    let line = contents
        .lines()
        .next()
        .ok_or_else(|| format!("{path} is empty"))?;
    Ok(serde_json::from_str(line)?)
}

fn decode_imsl(value: &Value) -> Result<ImslMessage, Box<dyn Error>> {
    match serde_json::to_string(value)?.decode_message()? {
        AcarsVdlm2Message::ImslMessage(imsl) => Ok(imsl),
        other => Err(format!("expected ImslMessage, got {other:?}").into()),
    }
}

fn reserialised(message: &ImslMessage) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_str(&message.to_string()?)?)
}

#[test]
fn irdm_without_station_id_decodes_and_round_trips_without_it() -> Result<(), Box<dyn Error>> {
    let mut value = first_line_value("test_files/irdm_docker")?;
    let source = value["source"]
        .as_object_mut()
        .ok_or("fixture source is not an object")?;
    assert!(source.remove("station_id").is_some(), "fixture must carry station_id");
    let line = serde_json::to_string(&value)?;

    let AcarsVdlm2Message::IrdmMessage(irdm) = line.decode_message()? else {
        return Err("expected IrdmMessage".into());
    };
    assert_eq!(irdm.source.station_id, None);

    let round_trip: Value = serde_json::from_str(&irdm.to_string()?)?;
    let round_trip_source = round_trip["source"]
        .as_object()
        .ok_or("round-tripped source is not an object")?;
    assert!(!round_trip_source.contains_key("station_id"));
    Ok(())
}

#[test]
fn imsl_float_timestamp_is_returned_by_get_time() -> Result<(), Box<dyn Error>> {
    let mut value = first_line_value("test_files/imsl_docker_acars")?;
    value["timestamp"] = json!(1_713_058_353.5);

    let imsl = decode_imsl(&value)?;
    assert_eq!(imsl.get_time(), Some(1_713_058_353.5));
    Ok(())
}

#[test]
fn imsl_integer_timestamp_is_still_accepted() -> Result<(), Box<dyn Error>> {
    let mut value = first_line_value("test_files/imsl_docker_acars")?;
    value["timestamp"] = json!(1_713_058_353_i64);

    let imsl = decode_imsl(&value)?;
    assert_eq!(imsl.get_time(), Some(1_713_058_353.0));
    Ok(())
}

#[test]
fn imsl_numeric_freq_decodes_and_round_trips_as_a_number() -> Result<(), Box<dyn Error>> {
    let mut value = first_line_value("test_files/imsl_docker_acars")?;
    value["freq"] = json!(1546.02);

    let imsl = decode_imsl(&value)?;
    assert_eq!(imsl.freq, Some(ImslFrequency::Number(1546.02)));
    assert_eq!(imsl.freq_string(), Some("1546.02".to_string()));
    assert_eq!(reserialised(&imsl)?["freq"], json!(1546.02));
    Ok(())
}

#[test]
fn imsl_string_freq_round_trips_as_a_string() -> Result<(), Box<dyn Error>> {
    let value = first_line_value("test_files/imsl_docker_acars")?;
    assert_eq!(value["freq"], json!("1546.020"), "fixture freq changed");

    let imsl = decode_imsl(&value)?;
    assert_eq!(imsl.freq, Some(ImslFrequency::Text("1546.020".to_string())));
    assert_eq!(imsl.freq_string(), Some("1546.020".to_string()));
    assert_eq!(reserialised(&imsl)?["freq"], json!("1546.020"));
    Ok(())
}
