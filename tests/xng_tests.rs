mod common;

use std::error::Error;
use chrono::{NaiveDate, NaiveDateTime};
use serde_json::{json, Value};
use acars_vdlm2_parser::{AcarsVdlm2Message, DecodeMessage};
use acars_vdlm2_parser::xng::{NewXngMessage, XngMessage};
use crate::common::{combine_files_of_message_type, load_files_of_message_type, MessageType, process_file_as_xng, read_test_file};

/// Loads the first line of the real off-air xng capture.
fn first_capture_line() -> Result<String, Box<dyn Error>> {
    read_test_file("test_files/xng_acars_capture")?
        .into_iter()
        .next()
        .ok_or_else(|| "xng capture file is empty".into())
}

/// Decodes a `Value` through the `AcarsVdlm2Message` enum, asserting it is the xng variant.
fn decode_xng_value(value: &Value) -> Result<AcarsVdlm2Message, Box<dyn Error>> {
    let message: AcarsVdlm2Message = serde_json::to_string(value)?.decode_message()?;
    assert!(matches!(message, AcarsVdlm2Message::XngMessage(_)), "Expected xng variant, got {:?}", message);
    Ok(message)
}

/// Serialises an `AcarsVdlm2Message` back to a `Value` for field inspection.
fn to_value(message: &AcarsVdlm2Message) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_str(&message.to_string()?)?)
}

/// This test will ingest contents from the xng sample files as a message per line to a `Vec<String>`.
/// Then it will cycle them into `Vec<XngMessage>` and back to `String`.
/// Every line is valid xng output, so it validates there are zero failures in either direction.
#[test]
fn test_xng_parsing() -> Result<(), Box<dyn Error>> {
    let xng_messages: Vec<String> = combine_files_of_message_type(MessageType::Xng)?;
    assert!(!xng_messages.is_empty(), "No xng test data was loaded");
    let mut failed_decodes: Vec<String> = Vec::new();
    for line in &xng_messages {
        match line.to_xng() {
            Err(error) => failed_decodes.push(format!("{}: {}", error, line)),
            Ok(message) => assert!(message.to_string().as_ref().err().is_none()),
        }
    }
    assert!(failed_decodes.is_empty(), "xng decode failures: {:#?}", failed_decodes);
    Ok(())
}

/// Every xng line must be identified as the `XngMessage` variant by the untagged enum.
#[test]
fn test_xng_decodes_to_xng_variant() -> Result<(), Box<dyn Error>> {
    for line in combine_files_of_message_type(MessageType::Xng)? {
        let message: AcarsVdlm2Message = line.decode_message()?;
        assert!(matches!(message, AcarsVdlm2Message::XngMessage(_)), "Line decoded to the wrong variant: {}", line);
    }
    Ok(())
}

/// Re-serialising every xng line must be semantically identical to the input, proving nothing is dropped or altered.
#[test]
fn test_xng_lossless_round_trip() -> Result<(), Box<dyn Error>> {
    for line in combine_files_of_message_type(MessageType::Xng)? {
        let original: Value = serde_json::from_str(&line)?;
        let direct: Value = serde_json::from_str(&line.to_xng()?.to_string()?)?;
        assert_eq!(original, direct, "XngMessage round trip altered: {}", line);
        let via_enum: Value = serde_json::from_str(&line.decode_message()?.to_string()?)?;
        assert_eq!(original, via_enum, "AcarsVdlm2Message round trip altered: {}", line);
    }
    Ok(())
}

/// `get_time` must return the RFC3339 timestamp as fractional epoch seconds.
#[test]
fn test_xng_get_time() -> Result<(), Box<dyn Error>> {
    let message: XngMessage = first_capture_line()?.to_xng()?;
    assert_eq!(message.timestamp, "2026-10-28T14:54:02.917095351Z");
    let expected: NaiveDateTime = NaiveDate::from_ymd_opt(2026, 10, 28)
        .and_then(|date| date.and_hms_nano_opt(14, 54, 2, 917_095_351))
        .ok_or("invalid expected date")?;
    let expected_secs: f64 = expected.and_utc().timestamp() as f64 + 917_095_351_f64 / 1e9;
    assert_eq!(message.get_time(), Some(expected_secs));

    let whole_second: XngMessage = XngMessage { timestamp: "2026-10-28T14:06:00Z".to_string(), ..Default::default() };
    let expected_whole: f64 = NaiveDate::from_ymd_opt(2026, 10, 28)
        .and_then(|date| date.and_hms_opt(14, 6, 0))
        .ok_or("invalid expected date")?
        .and_utc()
        .timestamp() as f64;
    assert_eq!(expected_whole, 1_793_196_360.0);
    assert_eq!(whole_second.get_time(), Some(expected_whole));

    let invalid: XngMessage = XngMessage { timestamp: "not a timestamp".to_string(), ..Default::default() };
    assert_eq!(invalid.get_time(), None);
    let mut cleared: XngMessage = message.clone();
    cleared.clear_time();
    assert_eq!(cleared.get_time(), None);

    let enum_message: AcarsVdlm2Message = first_capture_line()?.decode_message()?;
    assert_eq!(enum_message.get_time(), Some(expected_secs));
    Ok(())
}

/// Station name setting and clearing must update the ident, and clearing removes all per-receiver identity.
#[test]
fn test_xng_station_name() -> Result<(), Box<dyn Error>> {
    let mut message: AcarsVdlm2Message = first_capture_line()?.decode_message()?;
    message.set_station_name("NEW-STATION");
    let value: Value = to_value(&message)?;
    assert_eq!(value["source"]["station"]["ident"], "NEW-STATION");
    assert_eq!(value["source"]["station"]["id"], "da2e758e-7398-40d1-b69a-a500b2cc47be");
    assert!(value["source"].get("sdr").is_some());

    message.clear_station_name();
    let value: Value = to_value(&message)?;
    assert_eq!(value["source"]["station"]["ident"], "");
    assert_eq!(value["source"]["station"]["id"], "");
    assert!(value["source"].get("sdr").is_none());
    Ok(())
}

/// Proxy details must be added to and removed from `source.app`.
#[test]
fn test_xng_proxy_details() -> Result<(), Box<dyn Error>> {
    let mut message: AcarsVdlm2Message = first_capture_line()?.decode_message()?;
    message.set_proxy_details("acars_router", "1.2.3");
    let value: Value = to_value(&message)?;
    let app: &Value = &value["source"]["app"];
    assert_eq!(app["name"], "xng");
    assert_eq!(app["version"], "0.21.0");
    assert_eq!(app["proxied"], true);
    assert_eq!(app["proxied_by"], "acars_router");
    assert_eq!(app["acars_router_version"], "1.2.3");
    let uuid: String = app["acars_router_uuid"].as_str().ok_or("missing acars_router_uuid")?.to_string();
    assert_eq!(uuid.len(), 36);

    // Mirrors `AppDetails::proxy`: an existing uuid is retained.
    message.set_proxy_details("acars_router", "1.2.4");
    let value: Value = to_value(&message)?;
    assert_eq!(value["source"]["app"]["acars_router_uuid"], uuid.as_str());
    assert_eq!(value["source"]["app"]["acars_router_version"], "1.2.4");

    message.clear_proxy_details();
    let value: Value = to_value(&message)?;
    let app: &Value = &value["source"]["app"];
    for field in ["proxied", "proxied_by", "acars_router_version", "acars_router_uuid"] {
        assert!(app.get(field).is_none(), "{} still present after clear_proxy_details", field);
    }
    assert_eq!(app, &json!({"name": "xng", "version": "0.21.0"}));
    Ok(())
}

/// The signal, decode and channel clear functions must remove exactly their fields.
#[test]
fn test_xng_clear_fields() -> Result<(), Box<dyn Error>> {
    let mut value: Value = serde_json::from_str(&first_capture_line()?)?;
    value["signal"]["freq_skew_hz"] = json!(12.5);
    let base: AcarsVdlm2Message = decode_xng_value(&value)?;

    let mut message: AcarsVdlm2Message = base.clone();
    message.clear_freq_skew();
    let out: Value = to_value(&message)?;
    assert!(out["signal"].get("freq_skew_hz").is_none());
    assert!(out["signal"].get("rssi_db").is_some());

    let mut message: AcarsVdlm2Message = base.clone();
    message.clear_noise_level();
    let out: Value = to_value(&message)?;
    assert!(out["signal"].get("noise_db").is_none());
    assert!(out["signal"].get("snr_db").is_some());

    let mut message: AcarsVdlm2Message = base.clone();
    message.clear_sig_level();
    let out: Value = to_value(&message)?;
    assert!(out["signal"].get("rssi_db").is_none());
    assert!(out["signal"].get("snr_db").is_none());
    assert!(out["signal"].get("noise_db").is_some());

    let mut message: AcarsVdlm2Message = base.clone();
    message.clear_level();
    let out: Value = to_value(&message)?;
    assert!(out["signal"].get("rssi_db").is_none());
    assert!(out["signal"].get("snr_db").is_some());

    let mut message: AcarsVdlm2Message = base.clone();
    message.clear_error();
    let out: Value = to_value(&message)?;
    assert!(out["decode"].get("errors").is_none());
    assert!(out["decode"].get("fec_corrected").is_some());

    let mut message: AcarsVdlm2Message = base.clone();
    message.clear_octets_corrected_by_fec();
    let out: Value = to_value(&message)?;
    assert!(out["decode"].get("fec_corrected").is_none());
    assert!(out["decode"].get("errors").is_some());
    assert_eq!(out["decode"]["crc_ok"], true);

    let mut message: AcarsVdlm2Message = base.clone();
    message.clear_channel();
    let out: Value = to_value(&message)?;
    assert!(out["source"].get("channel").is_none());
    assert!(out["source"].get("sdr").is_some());

    let mut message: AcarsVdlm2Message = base.clone();
    message.clear_time();
    let out: Value = to_value(&message)?;
    assert_eq!(out["timestamp"], "");

    let mut message: AcarsVdlm2Message = base.clone();
    message.clear_hdr_bits_fixed();
    assert_eq!(to_value(&message)?, to_value(&base)?);
    Ok(())
}

/// Two receptions of the same message differing only in per-receiver details must serialise identically once cleared.
#[test]
fn test_xng_dedupe_clears() -> Result<(), Box<dyn Error>> {
    let first: Value = serde_json::from_str(&first_capture_line()?)?;
    let mut second: Value = first.clone();
    second["timestamp"] = json!("2026-10-28T14:54:03.000000001Z");
    second["source"]["station"]["id"] = json!("11111111-2222-3333-4444-555555555555");
    second["source"]["station"]["ident"] = json!("OTHER-STATION");
    second["source"]["sdr"] = json!({"id": "driver=airspy,serial=1", "driver": "soapysdr"});
    second["signal"] = json!({"rssi_db": -3.5, "snr_db": 30.25, "noise_db": -33.75, "freq_skew_hz": 88.0});
    second["decode"]["errors"] = json!(2);
    second["decode"]["fec_corrected"] = json!(7);
    second["source"]["channel"] = json!({"index": 9, "frequency_hz": 131550000, "sample_rate": 48000.0});

    let mut messages: Vec<AcarsVdlm2Message> = vec![decode_xng_value(&first)?, decode_xng_value(&second)?];
    assert_ne!(messages[0].to_string()?, messages[1].to_string()?);
    messages[1].set_proxy_details("acars_router", "1.2.3");
    for message in messages.iter_mut() {
        message.clear_time();
        message.clear_station_name();
        message.clear_sig_level();
        message.clear_noise_level();
        message.clear_freq_skew();
        message.clear_error();
        message.clear_octets_corrected_by_fec();
        message.clear_channel();
        message.clear_proxy_details();
    }
    assert_eq!(messages[0].to_string()?, messages[1].to_string()?);
    Ok(())
}

/// Unknown fields at the top level and in the body must be accepted and preserved.
#[test]
fn test_xng_unknown_fields_preserved() -> Result<(), Box<dyn Error>> {
    let mut value: Value = serde_json::from_str(&first_capture_line()?)?;
    value["future_top_level"] = json!({"nested": [1, 2, 3]});
    value["body"]["future_body_field"] = json!("hello");
    value["signal"]["future_signal"] = json!(1.25);
    value["source"]["app"]["future_app"] = json!(true);
    let message: AcarsVdlm2Message = decode_xng_value(&value)?;
    let out: Value = to_value(&message)?;
    assert_eq!(out["future_top_level"], json!({"nested": [1, 2, 3]}));
    assert_eq!(out["body"]["future_body_field"], "hello");
    assert_eq!(out, value);
    Ok(())
}

/// Existing non-xng fixtures must never be identified as xng.
#[test]
fn test_non_xng_fixtures_not_xng() -> Result<(), Box<dyn Error>> {
    for message_type in [MessageType::Acars, MessageType::Vdlm2, MessageType::Hfdl, MessageType::Imsl, MessageType::Irdm] {
        for line in combine_files_of_message_type(message_type)? {
            if let Ok(message) = line.decode_message() {
                assert!(!matches!(message, AcarsVdlm2Message::XngMessage(_)), "Non-xng line decoded as xng: {}", line);
            }
        }
    }
    Ok(())
}

/// Test for displaying the per-item result for xng messages, helpful when diagnosing parsing issues.
/// Marked as `#[ignore]` so it can be run separately as required.
#[test]
#[ignore]
fn show_xng_ingest() -> Result<(), Box<dyn Error>> {
    println!("Showing xng ingest errors");
    match load_files_of_message_type(MessageType::Xng) {
        Err(load_failed) => Err(load_failed),
        Ok(xng_files) => {
            for file in xng_files {
                println!("Testing the contents from file: {}", file.name);
                process_file_as_xng(&file.contents);
            }

            Ok(())
        }
    }
}
