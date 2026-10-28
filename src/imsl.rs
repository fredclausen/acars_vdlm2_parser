use std::fmt;

use serde::{Serialize, Deserialize};
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
    /// Any fields not modelled above, preserved on re-serialisation.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}
