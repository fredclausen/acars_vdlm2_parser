use std::fmt;

use serde::{Serialize, Deserialize};
use uuid::Uuid;
use crate::MessageResult;


/// Trait for performing a decode if you wish to apply it to types other than the defaults done in this library.
///
/// The originating data must be in JSON format and have support for providing a `str`, and will not consume the source.
///
/// This is intended for specifically decoding to `ImslMessage`.
pub trait NewImslMessage {
    fn to_imsl(&self) -> MessageResult<ImslMessage>;
}

/// Implementing `.to_imsl()` for the type `String`.
///
/// This does not consume the `String`.
impl NewImslMessage for String {
    fn to_imsl(&self) -> MessageResult<ImslMessage> {
        serde_json::from_str(self)
    }
}

/// Supporting `.to_imsl()` for the type `str`.
///
/// This does not consume the `str`.
impl NewImslMessage for str {
    fn to_imsl(&self) -> MessageResult<ImslMessage> {
        serde_json::from_str(self)
    }
}

impl ImslMessage {

    /// Converts `ImslMessage` to `String`.
    pub fn to_string(&self) -> MessageResult<String> {
        serde_json::to_string(self)
    }
    
    /// Converts `ImslMessage` to `String` and appends a `\n` to the end.
    pub fn to_string_newline(&self) -> MessageResult<String> {
        match serde_json::to_string(self) {
            Err(to_string_error) => Err(to_string_error),
            Ok(string) => Ok(format!("{}\n", string))
        }
    }

    /// Converts `ImslMessage` to a `String` encoded as bytes.
    ///
    /// The output is returned as a `Vec<u8>`.
    pub fn to_bytes(&self) -> MessageResult<Vec<u8>> {
        match self.to_string() {
            Err(conversion_failed) => Err(conversion_failed),
            Ok(string) => Ok(string.into_bytes())
        }
    }
    
    /// Converts `ImslMessage` to a `String` terminated with a `\n` and encoded as bytes.
    ///
    /// The output is returned as a `Vec<u8>`.
    pub fn to_bytes_newline(&self) -> MessageResult<Vec<u8>> {
        match self.to_string_newline() {
            Err(conversion_failed) => Err(conversion_failed),
            Ok(string) => Ok(string.into_bytes())
        }
    }

    pub fn get_time(&self) -> Option<f64> {
        self.timestamp.as_ref().and_then(serde_json::Number::as_f64)
    }

    /// Returns `freq` as a string, whether the source sent it as a string or a number.
    ///
    /// Consumers that need the frequency as text should use this rather than reading `freq`.
    /// ```
    /// use acars_vdlm2_parser::imsl::{ImslFrequency, ImslMessage};
    /// # fn main() -> Result<(), serde_json::Error> {
    /// let number: serde_json::Number = "1546.02".parse()?;
    /// let message = ImslMessage { freq: Some(ImslFrequency::Number(number)), ..Default::default() };
    /// assert_eq!(message.freq_string(), Some("1546.02".to_string()));
    /// # Ok(())
    /// # }
    /// ```
    pub fn freq_string(&self) -> Option<String> {
        self.freq.as_ref().map(ToString::to_string)
    }

    /// Sets `source.station_id` to the provided value.
    ///
    /// If the message has no `source`, an empty one is created to hold the station id.
    pub fn set_station_name(&mut self, station_name: &str) {
        self.source.get_or_insert_with(SourceBody::default).station_id = station_name.to_string();
    }

    /// Clears `source.station_id`.
    pub fn clear_station_name(&mut self) {
        if let Some(source) = self.source.as_mut() {
            source.station_id = String::new();
        }
    }

    /// Sets proxy details on `source.app` to the provided details and sets `proxied` to true.
    ///
    /// This mirrors `AppDetails::proxy()`, a new `acars_router_uuid` is only generated if one is not already present.
    /// If the message has no `source`, an empty one is created to hold the details.
    pub fn set_proxy_details(&mut self, proxied_by: &str, acars_router_version: &str) {
        let app: &mut AppBody = &mut self.source.get_or_insert_with(SourceBody::default).app;
        app.proxied = Some(true);
        app.proxied_by = Some(proxied_by.to_string());
        app.acars_router_version = Some(acars_router_version.to_string());
        if app.acars_router_uuid.is_none() {
            app.acars_router_uuid = Some(Uuid::new_v4().to_string());
        }
    }

    /// Clears the proxy details from `source.app`.
    pub fn clear_proxy_details(&mut self) {
        if let Some(source) = self.source.as_mut() {
            let app: &mut AppBody = &mut source.app;
            app.proxied = None;
            app.proxied_by = None;
            app.acars_router_version = None;
            app.acars_router_uuid = None;
        }
    }

    /// Clears `timestamp`.
    pub fn clear_time(&mut self) {
        self.timestamp = None;
    }

    /// Clears the receiver-measured `level`, which is not modelled and lives in `extra`.
    pub fn clear_level(&mut self) {
        self.extra.remove("level");
    }

    /// Clears the receiver-measured `level`, the only signal level IMSL reports.
    pub fn clear_sig_level(&mut self) {
        self.clear_level();
    }

    /// No-op: IMSL messages carry no error count.
    pub fn clear_error(&mut self) {}

    /// No-op: IMSL `freq` is the fixed downlink channel the message was sent on, so it is the same at
    /// every receiver and is kept. IMSL reports no frequency skew.
    pub fn clear_freq_skew(&mut self) {}

    /// No-op: IMSL messages carry no channel index.
    pub fn clear_channel(&mut self) {}

    /// No-op: IMSL messages do not report header bits fixed.
    pub fn clear_hdr_bits_fixed(&mut self) {}

    /// No-op: IMSL messages do not report a noise level.
    pub fn clear_noise_level(&mut self) {}

    /// No-op: IMSL messages do not report FEC corrections.
    pub fn clear_octets_corrected_by_fec(&mut self) {}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ImslMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bi: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_text: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub more_to_come: Option<bool>,
    pub msg_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plane_reg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal_unit: Option<SignalUnitBody>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceBody>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tak: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<serde_json::Number>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub freq: Option<ImslFrequency>,
    /// Any fields not modelled above, preserved on re-serialisation.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// The `freq` field of an `ImslMessage`, which SatDump emits as a string and other sources as a number.
///
/// Each form serialises back unchanged. Use `Display` (or `ImslMessage::freq_string`) for a text form.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum ImslFrequency {
    Text(String),
    Number(serde_json::Number),
}

impl fmt::Display for ImslFrequency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImslFrequency::Text(text) => f.write_str(text),
            ImslFrequency::Number(number) => write!(f, "{number}"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct SignalUnitBody {
    pub aes_id: i64,
    pub ges_id: i64,
    pub message_type: i64,
    pub no_of_bytes_in_last_su: i64,
    pub q_no: i64,
    pub ref_no: i64,
    pub seq_no: i64,
    /// Any fields not modelled above, preserved on re-serialisation.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct SourceBody {
    pub app: AppBody,
    pub station_id: String,
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
