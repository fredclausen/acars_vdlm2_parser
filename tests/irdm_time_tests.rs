use std::error::Error;
use std::fs;
use acars_vdlm2_parser::irdm::{AcarsBody, IrdmMessage, NewIrdmMessage};
use chrono::{NaiveDate, NaiveDateTime};

fn message_with_timestamp(timestamp: &str) -> IrdmMessage {
    IrdmMessage {
        acars: AcarsBody { timestamp: timestamp.to_string(), ..Default::default() },
        ..Default::default()
    }
}

fn expected_base() -> Option<f64> {
    let naive: NaiveDateTime = NaiveDate::from_ymd_opt(2024, 4, 14)?.and_hms_opt(2, 7, 30)?;
    Some(naive.and_utc().timestamp() as f64)
}

#[test]
fn test_irdm_get_time_naive_pre_5fd01de() {
    let expected = expected_base();
    assert_eq!(expected, Some(1_713_060_450.0));
    assert_eq!(message_with_timestamp("2024-04-14T02:07:30").get_time(), expected);
}

#[test]
fn test_irdm_get_time_utc_post_5fd01de() {
    assert_eq!(message_with_timestamp("2024-04-14T02:07:30Z").get_time(), expected_base());
}

#[test]
fn test_irdm_get_time_fractional() {
    assert_eq!(
        message_with_timestamp("2024-04-14T02:07:30.25Z").get_time(),
        expected_base().map(|base| base + 0.25)
    );
    assert_eq!(
        message_with_timestamp("2024-04-14T02:07:30.25").get_time(),
        expected_base().map(|base| base + 0.25)
    );
}

#[test]
fn test_irdm_get_time_offset() {
    assert_eq!(message_with_timestamp("2024-04-14T04:07:30+02:00").get_time(), expected_base());
}

#[test]
fn test_irdm_get_time_space_separator() {
    assert_eq!(message_with_timestamp("2024-04-14 02:07:30").get_time(), expected_base());
}

#[test]
fn test_irdm_get_time_invalid() {
    assert_eq!(message_with_timestamp("garbage").get_time(), None);
    assert_eq!(message_with_timestamp("").get_time(), None);
}

#[test]
fn test_irdm_fixtures_have_time() -> Result<(), Box<dyn Error>> {
    let mut checked = 0_usize;
    for path in ["test_files/irdm_bare", "test_files/irdm_docker"] {
        let contents = fs::read_to_string(path)?;
        for line in contents.lines().filter(|line| !line.trim().is_empty()) {
            let message = line.to_irdm()?;
            assert!(message.get_time().is_some(), "no time for {path}: {line}");
            checked += 1;
        }
    }
    assert!(checked > 0);
    Ok(())
}
