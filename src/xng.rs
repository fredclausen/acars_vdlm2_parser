use chrono::DateTime;
use serde::{Serialize, Deserialize};
use serde_json::{Map, Value};
use uuid::Uuid;
use crate::MessageResult;


/// Trait for performing a decode if you wish to apply it to types other than the defaults done in this library.
///
/// The originating data must be in JSON format and have support for providing a `str`, and will not consume the source.
///
/// This is intended for specifically decoding to `XngMessage`.
pub trait NewXngMessage {
    fn to_xng(&self) -> MessageResult<XngMessage>;
}

/// Implementing `.to_xng()` for the type `String`.
///
/// This does not consume the `String`.
impl NewXngMessage for String {
    fn to_xng(&self) -> MessageResult<XngMessage> {
        serde_json::from_str(self)
    }
}

/// Supporting `.to_xng()` for the type `str`.
///
/// This does not consume the `str`.
impl NewXngMessage for str {
    fn to_xng(&self) -> MessageResult<XngMessage> {
        serde_json::from_str(self)
    }
}

impl XngMessage {

    /// Converts `XngMessage` to `String`.
    pub fn to_string(&self) -> MessageResult<String> {
        serde_json::to_string(self)
    }

    /// Converts `XngMessage` to `String` and appends a `\n` to the end.
    pub fn to_string_newline(&self) -> MessageResult<String> {
        match serde_json::to_string(self) {
            Err(to_string_error) => Err(to_string_error),
            Ok(string) => Ok(format!("{}\n", string))
        }
    }

    /// Converts `XngMessage` to a `String` encoded as bytes.
    ///
    /// The output is returned as a `Vec<u8>`.
    pub fn to_bytes(&self) -> MessageResult<Vec<u8>> {
        match self.to_string() {
            Err(conversion_failed) => Err(conversion_failed),
            Ok(string) => Ok(string.into_bytes())
        }
    }

    /// Converts `XngMessage` to a `String` terminated with a `\n` and encoded as bytes.
    ///
    /// The output is returned as a `Vec<u8>`.
    pub fn to_bytes_newline(&self) -> MessageResult<Vec<u8>> {
        match self.to_string_newline() {
            Err(conversion_failed) => Err(conversion_failed),
            Ok(string) => Ok(string.into_bytes())
        }
    }

    /// Retrieves the RFC3339 `timestamp` as seconds since the Unix epoch, including the sub-second fraction.
    ///
    /// Returns `None` if the timestamp cannot be parsed.
    /// ```
    /// use acars_vdlm2_parser::xng::XngMessage;
    /// let message: XngMessage = XngMessage { timestamp: "1970-01-01T00:00:01.5Z".to_string(), ..Default::default() };
    /// assert_eq!(message.get_time(), Some(1.5));
    /// ```
    pub fn get_time(&self) -> Option<f64> {
        let parsed = DateTime::parse_from_rfc3339(&self.timestamp).ok()?;
        Some(parsed.timestamp() as f64 + f64::from(parsed.timestamp_subsec_nanos()) / 1e9)
    }

    /// Clears the `timestamp` field.
    pub fn clear_time(&mut self) {
        self.timestamp = String::new();
    }

    /// Sets the station ident to the provided value.
    pub fn set_station_name(&mut self, station_name: &str) {
        self.source.station.ident = station_name.to_string();
    }

    /// Clears all per-receiver identity: the station ident, the station id and the SDR details.
    /// ```
    /// use acars_vdlm2_parser::xng::XngMessage;
    /// let mut message: XngMessage = XngMessage::default();
    /// message.set_station_name("test_station");
    /// assert_eq!(message.source.station.ident, "test_station");
    /// message.clear_station_name();
    /// assert!(message.source.station.ident.is_empty());
    /// ```
    pub fn clear_station_name(&mut self) {
        self.source.station.ident = String::new();
        self.source.station.id = String::new();
        self.source.sdr = None;
    }

    /// Sets proxy details to the provided details and sets `proxied` to true.
    ///
    /// This mirrors `AppDetails::proxy()`, a new `acars_router_uuid` is only generated if one is not already present.
    pub fn set_proxy_details(&mut self, proxied_by: &str, acars_router_version: &str) {
        let app: &mut XngApp = &mut self.source.app;
        app.proxied = Some(true);
        app.proxied_by = Some(proxied_by.to_string());
        app.acars_router_version = Some(acars_router_version.to_string());
        if app.acars_router_uuid.is_none() {
            app.acars_router_uuid = Some(Uuid::new_v4().to_string());
        }
    }

    /// Clears any proxy details that may be set.
    pub fn clear_proxy_details(&mut self) {
        let app: &mut XngApp = &mut self.source.app;
        app.proxied = None;
        app.proxied_by = None;
        app.acars_router_version = None;
        app.acars_router_uuid = None;
    }

    /// Clears the `freq_skew_hz` signal field.
    pub fn clear_freq_skew(&mut self) {
        self.signal.freq_skew_hz = None;
    }

    /// No-op: xng does not report header bits fixed separately, FEC corrections are covered by `clear_octets_corrected_by_fec`.
    pub fn clear_hdr_bits_fixed(&mut self) {}

    /// Clears the `noise_db` signal field.
    pub fn clear_noise_level(&mut self) {
        self.signal.noise_db = None;
    }

    /// Clears the `rssi_db` and `snr_db` signal fields.
    pub fn clear_sig_level(&mut self) {
        self.signal.rssi_db = None;
        self.signal.snr_db = None;
    }

    /// Clears the `rssi_db` signal field.
    pub fn clear_level(&mut self) {
        self.signal.rssi_db = None;
    }

    /// Clears the `errors` decode field.
    pub fn clear_error(&mut self) {
        self.decode.errors = None;
    }

    /// Clears the `fec_corrected` decode field.
    pub fn clear_octets_corrected_by_fec(&mut self) {
        self.decode.fec_corrected = None;
    }

    /// Clears the `channel` source field.
    pub fn clear_channel(&mut self) {
        self.source.channel = None;
    }
}

/// A single xng message envelope, as emitted for every air interface.
///
/// Parsing is lenient and lossless: unknown fields are retained in the `extra`/`fields` maps and re-serialised.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct XngMessage {
    /// Air interface the message was received on, e.g. `vdl2`, `hfdl`, `acars_poa`.
    pub mode: String,
    /// RFC3339 reception timestamp.
    pub timestamp: String,
    /// Frequency the message was received on, in Hz.
    pub frequency_hz: u64,
    /// Signal measurements.
    #[serde(default)]
    pub signal: XngSignal,
    /// Decoder status.
    #[serde(default)]
    pub decode: XngDecode,
    /// Decoded message body.
    pub body: XngBody,
    /// Raw frame as a lowercase hex string.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub raw: Option<String>,
    /// Where the message came from.
    pub source: XngSource,
    /// Any unknown top-level fields.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Signal measurements for an `XngMessage`.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct XngSignal {
    /// Received signal strength in dB.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub rssi_db: Option<f64>,
    /// Signal to noise ratio in dB.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub snr_db: Option<f64>,
    /// Noise level in dB.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub noise_db: Option<f64>,
    /// Frequency skew in Hz.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub freq_skew_hz: Option<f64>,
    /// Receive time in 12 MHz ticks.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub rx_ticks_12mhz: Option<u64>,
    /// Any unknown signal fields.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Decoder status for an `XngMessage`.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct XngDecode {
    /// Whether the CRC check passed.
    #[serde(default)]
    pub crc_ok: bool,
    /// Number of units corrected by FEC.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub fec_corrected: Option<u32>,
    /// Number of errors detected.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub errors: Option<u32>,
    /// Any unknown decode fields.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Decoded message body for an `XngMessage`, free-form per `type`.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct XngBody {
    /// Body type, e.g. `acars`, `vdl2`, `hfdl`, `aero`, `iridium`.
    #[serde(rename = "type")]
    pub body_type: String,
    /// All remaining body fields.
    #[serde(flatten)]
    pub fields: Map<String, Value>,
}

/// Origin details for an `XngMessage`.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct XngSource {
    /// The receiving station.
    pub station: XngStation,
    /// The decoding application.
    pub app: XngApp,
    /// SDR details.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sdr: Option<Value>,
    /// Channel details.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub channel: Option<Value>,
    /// Any unknown source fields.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Receiving station details for an `XngMessage`.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct XngStation {
    /// Station UUID.
    pub id: String,
    /// Station ident.
    pub ident: String,
    /// Any unknown station fields.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Decoding application details for an `XngMessage`, including acars_router proxy details.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct XngApp {
    /// Application name.
    pub name: String,
    /// Application version.
    pub version: String,
    /// Set to true once proxied by acars_router.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub proxied: Option<bool>,
    /// Name of the proxying application.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub proxied_by: Option<String>,
    /// Version of the proxying acars_router.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub acars_router_version: Option<String>,
    /// UUID assigned by the proxying acars_router.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub acars_router_uuid: Option<String>,
    /// Any unknown app fields.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
