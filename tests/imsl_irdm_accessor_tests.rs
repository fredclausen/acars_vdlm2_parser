use std::error::Error;
use std::fs;

use acars_vdlm2_parser::{AcarsVdlm2Message, DecodeMessage};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn Error>>;

fn first_line(path: &str) -> Result<String, Box<dyn Error>> {
    let contents = fs::read_to_string(path)?;
    contents
        .lines()
        .next()
        .map(str::to_string)
        .ok_or_else(|| format!("{path}: empty fixture").into())
}

fn decode(value: &Value) -> Result<AcarsVdlm2Message, Box<dyn Error>> {
    Ok(value.to_string().decode_message()?)
}

fn to_value(message: &AcarsVdlm2Message) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_str(&message.to_string()?)?)
}

fn imsl_docker() -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_str(&first_line("test_files/imsl_docker_acars")?)?)
}

fn irdm_docker() -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_str(&first_line("test_files/irdm_docker")?)?)
}

fn assert_imsl(message: &AcarsVdlm2Message) -> TestResult {
    match message {
        AcarsVdlm2Message::ImslMessage(_) => Ok(()),
        other => Err(format!("expected ImslMessage, got {other:?}").into()),
    }
}

fn assert_irdm(message: &AcarsVdlm2Message) -> TestResult {
    match message {
        AcarsVdlm2Message::IrdmMessage(_) => Ok(()),
        other => Err(format!("expected IrdmMessage, got {other:?}").into()),
    }
}

/// Every `clear_*` method acars_router calls before hashing a message for deduplication.
fn clear_router_set(message: &mut AcarsVdlm2Message) {
    message.clear_proxy_details();
    message.clear_error();
    message.clear_level();
    message.clear_station_name();
    message.clear_time();
    message.clear_channel();
    message.clear_freq_skew();
    message.clear_hdr_bits_fixed();
    message.clear_noise_level();
    message.clear_octets_corrected_by_fec();
    message.clear_sig_level();
}

#[test]
fn set_station_name_overrides_imsl_station_id() -> TestResult {
    let mut message = decode(&imsl_docker()?)?;
    assert_imsl(&message)?;
    message.set_station_name("OVERRIDE");
    assert_eq!(to_value(&message)?["source"]["station_id"], json!("OVERRIDE"));
    Ok(())
}

#[test]
fn set_station_name_overrides_irdm_station_id() -> TestResult {
    let mut message = decode(&irdm_docker()?)?;
    assert_irdm(&message)?;
    message.set_station_name("OVERRIDE");
    assert_eq!(to_value(&message)?["source"]["station_id"], json!("OVERRIDE"));
    Ok(())
}

#[test]
fn set_station_name_adds_absent_irdm_station_id() -> TestResult {
    let mut original = irdm_docker()?;
    original["source"]
        .as_object_mut()
        .ok_or("source is not an object")?
        .remove("station_id");
    let mut message = decode(&original)?;
    assert_irdm(&message)?;
    assert!(to_value(&message)?["source"].get("station_id").is_none());
    message.set_station_name("OVERRIDE");
    assert_eq!(to_value(&message)?["source"]["station_id"], json!("OVERRIDE"));
    Ok(())
}

fn check_proxy_details(original: &Value, app_path: &[&str]) -> TestResult {
    let mut message = decode(original)?;
    let app = |value: &Value| -> Value {
        app_path.iter().fold(value.clone(), |current, key| current[*key].clone())
    };

    message.set_proxy_details("test_proxy", "9.9.9");
    let proxied = to_value(&message)?;
    let proxied_app = app(&proxied);
    assert_eq!(proxied_app["proxied"], json!(true));
    assert_eq!(proxied_app["proxied_by"], json!("test_proxy"));
    assert_eq!(proxied_app["acars_router_version"], json!("9.9.9"));
    let uuid = proxied_app["acars_router_uuid"]
        .as_str()
        .ok_or("acars_router_uuid missing")?
        .to_string();
    assert_eq!(uuid.len(), 36);

    message.set_proxy_details("other_proxy", "1.0.0");
    let reproxied_app = app(&to_value(&message)?);
    assert_eq!(reproxied_app["proxied_by"], json!("other_proxy"));
    assert_eq!(reproxied_app["acars_router_version"], json!("1.0.0"));
    assert_eq!(reproxied_app["acars_router_uuid"], json!(uuid));

    message.clear_proxy_details();
    let cleared = to_value(&message)?;
    let cleared_app = app(&cleared);
    for key in ["proxied", "proxied_by", "acars_router_version", "acars_router_uuid"] {
        assert!(cleared_app.get(key).is_none(), "{key} still present");
    }
    assert_eq!(&cleared, original);
    Ok(())
}

#[test]
fn proxy_details_round_trip_on_imsl_source_app() -> TestResult {
    let original = imsl_docker()?;
    assert_imsl(&decode(&original)?)?;
    check_proxy_details(&original, &["source", "app"])
}

#[test]
fn proxy_details_round_trip_on_irdm_app() -> TestResult {
    let original = irdm_docker()?;
    assert_irdm(&decode(&original)?)?;
    check_proxy_details(&original, &["app"])
}

fn check_dedupe(first: &Value, second: &Value) -> TestResult {
    let mut first_message = decode(first)?;
    let mut second_message = decode(second)?;
    assert_ne!(first_message.to_string()?, second_message.to_string()?);
    clear_router_set(&mut first_message);
    clear_router_set(&mut second_message);
    assert_eq!(first_message.to_string()?, second_message.to_string()?);
    Ok(())
}

#[test]
fn imsl_copies_from_two_receivers_dedupe_after_clearing() -> TestResult {
    let first = imsl_docker()?;
    assert!(first.get("level").is_some());
    let mut second = first.clone();
    second["timestamp"] = json!(1713058999);
    second["source"]["station_id"] = json!("OTHER-STATION");
    second["level"] = json!("-20.0");
    check_dedupe(&first, &second)
}

#[test]
fn irdm_copies_from_two_receivers_dedupe_after_clearing() -> TestResult {
    let first = irdm_docker()?;
    assert!(first.get("freq").is_some() && first.get("level").is_some());
    let mut second = first.clone();
    second["acars"]["timestamp"] = json!("2024-04-14T01:45:25");
    second["acars"]["errors"] = json!(2);
    second["source"]["station_id"] = json!("OTHER-STATION");
    second["freq"] = json!(1625712000.5);
    second["level"] = json!(-55.5);
    check_dedupe(&first, &second)
}

#[test]
fn clearing_does_not_remove_irdm_header() -> TestResult {
    let original: Value = serde_json::from_str(
        fs::read_to_string("test_files/irdm_docker")?
            .lines()
            .find(|line| !line.contains(r#""header": """#))
            .ok_or("no IRDM line with a header")?,
    )?;
    let mut message = decode(&original)?;
    clear_router_set(&mut message);
    assert_eq!(to_value(&message)?["header"], original["header"]);
    Ok(())
}

#[test]
fn get_time_works_before_clear_time() -> TestResult {
    let mut imsl = decode(&imsl_docker()?)?;
    assert_eq!(imsl.get_time(), Some(1713058872.0));
    imsl.clear_time();
    assert_eq!(imsl.get_time(), None);

    let mut irdm = decode(&irdm_docker()?)?;
    assert_eq!(irdm.get_time(), Some(1713059124.0));
    irdm.clear_time();
    assert_eq!(irdm.get_time(), None);
    Ok(())
}
