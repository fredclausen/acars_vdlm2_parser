use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use crate::MessageResult;


/// Trait for performing a decode if you wish to apply it to types other than the defaults done in this library.
///
/// The originating data must be in JSON format and have support for providing a `str`, and will not consume the source.
///
/// This is intended for specifically decoding to `ImslMessage`.
pub trait NewIrdmMessage {
    fn to_irdm(&self) -> MessageResult<IrdmMessage>;
}

/// Implementing `.to_irdm()` for the type `String`.
///
/// This does not consume the `String`.
impl NewIrdmMessage for String {
    fn to_irdm(&self) -> MessageResult<IrdmMessage> {
        serde_json::from_str(self)
    }
}

/// Supporting `.to_irdm()` for the type `str`.
///
/// This does not consume the `str`.
impl NewIrdmMessage for str {
    fn to_irdm(&self) -> MessageResult<IrdmMessage> {
        serde_json::from_str(self)
    }
}

impl IrdmMessage {

    /// Converts `IrdmMessage` to `String`.
    pub fn to_string(&self) -> MessageResult<String> {
        serde_json::to_string(self)
    }
    
    /// Converts `IrdmMessage` to `String` and appends a `\n` to the end.
    pub fn to_string_newline(&self) -> MessageResult<String> {
        match serde_json::to_string(self) {
            Err(to_string_error) => Err(to_string_error),
            Ok(string) => Ok(format!("{}\n", string))
        }
    }

    /// Converts `IrdmMessage` to a `String` encoded as bytes.
    ///
    /// The output is returned as a `Vec<u8>`.
    pub fn to_bytes(&self) -> MessageResult<Vec<u8>> {
        match self.to_string() {
            Err(conversion_failed) => Err(conversion_failed),
            Ok(string) => Ok(string.into_bytes())
        }
    }
    
    /// Converts `IrdmMessage` to a `String` terminated with a `\n` and encoded as bytes.
    ///
    /// The output is returned as a `Vec<u8>`.
    pub fn to_bytes_newline(&self) -> MessageResult<Vec<u8>> {
        match self.to_string_newline() {
            Err(conversion_failed) => Err(conversion_failed),
            Ok(string) => Ok(string.into_bytes())
        }
    }

    /// Retrieves `acars.timestamp` as seconds since the Unix epoch, including the sub-second fraction.
    ///
    /// iridium-toolkit emitted a naive timestamp without a zone (`YYYY-MM-DDTHH:MM:SS`) before
    /// upstream commit 5fd01de (July 2024), and a UTC timestamp (`YYYY-MM-DDTHH:MM:SSZ`) after it.
    /// RFC 3339 timestamps (including offsets and fractions) are parsed as such; naive timestamps,
    /// with either a `T` or a space separator, are interpreted as UTC.
    ///
    /// Returns `None` if the timestamp cannot be parsed.
    /// ```
    /// use acars_vdlm2_parser::irdm::{AcarsBody, IrdmMessage};
    /// let message = IrdmMessage {
    ///     acars: AcarsBody { timestamp: "1970-01-01T00:00:01.5".to_string(), ..Default::default() },
    ///     ..Default::default()
    /// };
    /// assert_eq!(message.get_time(), Some(1.5));
    /// ```
    pub fn get_time(&self) -> Option<f64> {
        let timestamp = self.acars.timestamp.as_str();
        let parsed = match DateTime::parse_from_rfc3339(timestamp) {
            Ok(with_offset) => with_offset.with_timezone(&Utc),
            Err(_) => {
                let naive = timestamp.strip_suffix('Z').unwrap_or(timestamp);
                NaiveDateTime::parse_from_str(naive, "%Y-%m-%dT%H:%M:%S%.f")
                    .or_else(|_| NaiveDateTime::parse_from_str(naive, "%Y-%m-%d %H:%M:%S%.f"))
                    .ok()?
                    .and_utc()
            }
        };
        Some(parsed.timestamp() as f64 + f64::from(parsed.timestamp_subsec_nanos()) / 1e9)
    }

    /// Sets `source.station_id` to the provided value.
    pub fn set_station_name(&mut self, station_name: &str) {
        self.source.station_id = Some(station_name.to_string());
    }

    /// Clears `source.station_id`.
    pub fn clear_station_name(&mut self) {
        self.source.station_id = None;
    }

    /// Sets proxy details on the top-level `app` to the provided details and sets `proxied` to true.
    ///
    /// This mirrors `AppDetails::proxy()`, a new `acars_router_uuid` is only generated if one is not already present.
    pub fn set_proxy_details(&mut self, proxied_by: &str, acars_router_version: &str) {
        let app: &mut AppBody = &mut self.app;
        app.proxied = Some(true);
        app.proxied_by = Some(proxied_by.to_string());
        app.acars_router_version = Some(acars_router_version.to_string());
        if app.acars_router_uuid.is_none() {
            app.acars_router_uuid = Some(Uuid::new_v4().to_string());
        }
    }

    /// Clears the proxy details from the top-level `app`.
    pub fn clear_proxy_details(&mut self) {
        let app: &mut AppBody = &mut self.app;
        app.proxied = None;
        app.proxied_by = None;
        app.acars_router_version = None;
        app.acars_router_uuid = None;
    }

    /// Clears `acars.timestamp`.
    ///
    /// `header` is left alone: it is the hex of the bytes preceding the ACARS block in the
    /// transmitted frame, so it is message content rather than receiver timing.
    pub fn clear_time(&mut self) {
        self.acars.timestamp = String::new();
    }

    /// Clears the receiver-measured `level`.
    pub fn clear_level(&mut self) {
        self.level = None;
    }

    /// Clears the receiver-measured `level`, the only signal level IRDM reports.
    pub fn clear_sig_level(&mut self) {
        self.clear_level();
    }

    /// Resets `acars.errors` to zero. The field is required, so it cannot be removed.
    ///
    /// The error count describes the copy a given receiver decoded, not the message itself.
    pub fn clear_error(&mut self) {
        self.acars.errors = 0;
    }

    /// Clears `freq`.
    ///
    /// An Iridium burst's measured frequency includes the satellite Doppler offset, so it differs per
    /// receiver and must not take part in deduplication.
    pub fn clear_freq_skew(&mut self) {
        self.freq = None;
    }

    /// No-op: IRDM messages carry no channel index.
    pub fn clear_channel(&mut self) {}

    /// No-op: IRDM messages do not report header bits fixed.
    pub fn clear_hdr_bits_fixed(&mut self) {}

    /// No-op: IRDM messages do not report a noise level.
    pub fn clear_noise_level(&mut self) {}

    /// No-op: IRDM messages do not report FEC corrections.
    pub fn clear_octets_corrected_by_fec(&mut self) {}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct IrdmMessage {
    pub app: AppBody,
    pub source: SourceBody,
    pub acars: AcarsBody,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub freq: Option<serde_json::Number>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<serde_json::Number>,
    pub header: String,
    /// Any fields not modelled above, preserved on re-serialisation.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct AppBody {
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub proxied: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub proxied_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub acars_router_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub acars_router_uuid: Option<String>,
    /// Any fields not modelled above, preserved on re-serialisation.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct SourceBody {
    pub transport: String,
    pub protocol: String,
    /// iridium-toolkit omits this unless it is run with `--station`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub station_id: Option<String>,
    /// Any fields not modelled above, preserved on re-serialisation.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct AcarsBody {
    pub timestamp: String,
    pub errors: i64,
    pub link_direction: String,
    pub block_end: bool,
    pub mode: String,
    pub tail: String,
    pub label: String,
    pub block_id: String,
    pub ack: String,
    pub text: String,
    /// Any fields not modelled above, preserved on re-serialisation.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}
