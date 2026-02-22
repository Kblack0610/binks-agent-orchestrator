//! Parameter types for ADB MCP tools

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};

// ============================================================================
// Lenient Numeric Parsing (handles string-encoded numbers from LLM clients)
// ============================================================================

/// Deserialize an i32 that can be a number or string representation
fn deserialize_lenient_i32<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, Visitor};

    struct LenientI32Visitor;

    impl<'de> Visitor<'de> for LenientI32Visitor {
        type Value = i32;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("an integer or string-encoded integer")
        }

        fn visit_i32<E>(self, value: i32) -> Result<i32, E> {
            Ok(value)
        }

        fn visit_i64<E>(self, value: i64) -> Result<i32, E>
        where
            E: de::Error,
        {
            i32::try_from(value).map_err(|_| de::Error::custom(format!("i64 {value} out of i32 range")))
        }

        fn visit_u64<E>(self, value: u64) -> Result<i32, E>
        where
            E: de::Error,
        {
            i32::try_from(value).map_err(|_| de::Error::custom(format!("u64 {value} out of i32 range")))
        }

        fn visit_f64<E>(self, value: f64) -> Result<i32, E>
        where
            E: de::Error,
        {
            if value.fract() == 0.0 && value >= i32::MIN as f64 && value <= i32::MAX as f64 {
                Ok(value as i32)
            } else {
                Err(de::Error::custom(format!("f64 {value} cannot be represented as i32")))
            }
        }

        fn visit_str<E>(self, value: &str) -> Result<i32, E>
        where
            E: de::Error,
        {
            value.parse::<i32>().map_err(|_| {
                de::Error::custom(format!("invalid string for i32: '{value}'"))
            })
        }
    }

    deserializer.deserialize_any(LenientI32Visitor)
}

/// Deserialize a u32 that can be a number or string representation
fn deserialize_lenient_u32<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, Visitor};

    struct LenientU32Visitor;

    impl<'de> Visitor<'de> for LenientU32Visitor {
        type Value = u32;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a non-negative integer or string-encoded integer")
        }

        fn visit_i64<E>(self, value: i64) -> Result<u32, E>
        where
            E: de::Error,
        {
            u32::try_from(value).map_err(|_| de::Error::custom(format!("i64 {value} out of u32 range")))
        }

        fn visit_u64<E>(self, value: u64) -> Result<u32, E>
        where
            E: de::Error,
        {
            u32::try_from(value).map_err(|_| de::Error::custom(format!("u64 {value} out of u32 range")))
        }

        fn visit_f64<E>(self, value: f64) -> Result<u32, E>
        where
            E: de::Error,
        {
            if value.fract() == 0.0 && value >= 0.0 && value <= u32::MAX as f64 {
                Ok(value as u32)
            } else {
                Err(de::Error::custom(format!("f64 {value} cannot be represented as u32")))
            }
        }

        fn visit_str<E>(self, value: &str) -> Result<u32, E>
        where
            E: de::Error,
        {
            value.parse::<u32>().map_err(|_| {
                de::Error::custom(format!("invalid string for u32: '{value}'"))
            })
        }
    }

    deserializer.deserialize_any(LenientU32Visitor)
}

/// Deserialize an Option<u32> with lenient parsing
fn deserialize_lenient_u32_opt<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, Visitor};

    struct LenientU32OptVisitor;

    impl<'de> Visitor<'de> for LenientU32OptVisitor {
        type Value = Option<u32>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("null, a non-negative integer, or string-encoded integer")
        }

        fn visit_none<E>(self) -> Result<Option<u32>, E> {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Option<u32>, E> {
            Ok(None)
        }

        fn visit_i64<E>(self, value: i64) -> Result<Option<u32>, E>
        where
            E: de::Error,
        {
            u32::try_from(value)
                .map(Some)
                .map_err(|_| de::Error::custom(format!("i64 {value} out of u32 range")))
        }

        fn visit_u64<E>(self, value: u64) -> Result<Option<u32>, E>
        where
            E: de::Error,
        {
            u32::try_from(value)
                .map(Some)
                .map_err(|_| de::Error::custom(format!("u64 {value} out of u32 range")))
        }

        fn visit_f64<E>(self, value: f64) -> Result<Option<u32>, E>
        where
            E: de::Error,
        {
            if value.fract() == 0.0 && value >= 0.0 && value <= u32::MAX as f64 {
                Ok(Some(value as u32))
            } else {
                Err(de::Error::custom(format!("f64 {value} cannot be represented as u32")))
            }
        }

        fn visit_str<E>(self, value: &str) -> Result<Option<u32>, E>
        where
            E: de::Error,
        {
            value.parse::<u32>().map(Some).map_err(|_| {
                de::Error::custom(format!("invalid string for u32: '{value}'"))
            })
        }
    }

    deserializer.deserialize_any(LenientU32OptVisitor)
}

/// Deserialize an Option<u64> with lenient parsing
fn deserialize_lenient_u64_opt<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, Visitor};

    struct LenientU64OptVisitor;

    impl<'de> Visitor<'de> for LenientU64OptVisitor {
        type Value = Option<u64>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("null, a non-negative integer, or string-encoded integer")
        }

        fn visit_none<E>(self) -> Result<Option<u64>, E> {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Option<u64>, E> {
            Ok(None)
        }

        fn visit_i64<E>(self, value: i64) -> Result<Option<u64>, E>
        where
            E: de::Error,
        {
            u64::try_from(value)
                .map(Some)
                .map_err(|_| de::Error::custom(format!("i64 {value} out of u64 range")))
        }

        fn visit_u64<E>(self, value: u64) -> Result<Option<u64>, E> {
            Ok(Some(value))
        }

        fn visit_f64<E>(self, value: f64) -> Result<Option<u64>, E>
        where
            E: de::Error,
        {
            if value.fract() == 0.0 && value >= 0.0 && value <= u64::MAX as f64 {
                Ok(Some(value as u64))
            } else {
                Err(de::Error::custom(format!("f64 {value} cannot be represented as u64")))
            }
        }

        fn visit_str<E>(self, value: &str) -> Result<Option<u64>, E>
        where
            E: de::Error,
        {
            value.parse::<u64>().map(Some).map_err(|_| {
                de::Error::custom(format!("invalid string for u64: '{value}'"))
            })
        }
    }

    deserializer.deserialize_any(LenientU64OptVisitor)
}

/// Deserialize an Option<u8> with lenient parsing
fn deserialize_lenient_u8_opt<'de, D>(deserializer: D) -> Result<Option<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, Visitor};

    struct LenientU8OptVisitor;

    impl<'de> Visitor<'de> for LenientU8OptVisitor {
        type Value = Option<u8>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("null, a small integer (0-255), or string-encoded integer")
        }

        fn visit_none<E>(self) -> Result<Option<u8>, E> {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Option<u8>, E> {
            Ok(None)
        }

        fn visit_i64<E>(self, value: i64) -> Result<Option<u8>, E>
        where
            E: de::Error,
        {
            u8::try_from(value)
                .map(Some)
                .map_err(|_| de::Error::custom(format!("i64 {value} out of u8 range (0-255)")))
        }

        fn visit_u64<E>(self, value: u64) -> Result<Option<u8>, E>
        where
            E: de::Error,
        {
            u8::try_from(value)
                .map(Some)
                .map_err(|_| de::Error::custom(format!("u64 {value} out of u8 range (0-255)")))
        }

        fn visit_f64<E>(self, value: f64) -> Result<Option<u8>, E>
        where
            E: de::Error,
        {
            if value.fract() == 0.0 && (0.0..=255.0).contains(&value) {
                Ok(Some(value as u8))
            } else {
                Err(de::Error::custom(format!("f64 {value} cannot be represented as u8")))
            }
        }

        fn visit_str<E>(self, value: &str) -> Result<Option<u8>, E>
        where
            E: de::Error,
        {
            value.parse::<u8>().map(Some).map_err(|_| {
                de::Error::custom(format!("invalid string for u8: '{value}'"))
            })
        }
    }

    deserializer.deserialize_any(LenientU8OptVisitor)
}

// ============================================================================
// Parameter Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DevicesParams {}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CropRegion {
    #[schemars(description = "X offset of the crop region (pixels from left)")]
    #[serde(deserialize_with = "deserialize_lenient_u32")]
    pub x: u32,

    #[schemars(description = "Y offset of the crop region (pixels from top)")]
    #[serde(deserialize_with = "deserialize_lenient_u32")]
    pub y: u32,

    #[schemars(description = "Width of the crop region in pixels")]
    #[serde(deserialize_with = "deserialize_lenient_u32")]
    pub width: u32,

    #[schemars(description = "Height of the crop region in pixels")]
    #[serde(deserialize_with = "deserialize_lenient_u32")]
    pub height: u32,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ScreenshotParams {
    #[schemars(description = "Device serial number (optional, auto-selects if only one device)")]
    #[serde(default)]
    pub device: Option<String>,

    #[schemars(description = "File path to save screenshot (optional, returns base64 if omitted)")]
    #[serde(default)]
    pub output_path: Option<String>,

    #[schemars(description = "Output format: \"png\" or \"jpeg\" (default: \"jpeg\")")]
    #[serde(default)]
    pub format: Option<String>,

    #[schemars(description = "JPEG quality 1-100 (default: 80, ignored for PNG)")]
    #[serde(default, deserialize_with = "deserialize_lenient_u8_opt")]
    pub quality: Option<u8>,

    #[schemars(
        description = "Max output width in pixels, preserving aspect ratio (default: 1024, 0 = no resize)"
    )]
    #[serde(default, deserialize_with = "deserialize_lenient_u32_opt")]
    pub max_width: Option<u32>,

    #[schemars(
        description = "Max output height in pixels, preserving aspect ratio (default: 1920, 0 = no resize)"
    )]
    #[serde(default, deserialize_with = "deserialize_lenient_u32_opt")]
    pub max_height: Option<u32>,

    #[schemars(description = "Crop region to extract before resizing")]
    #[serde(default)]
    pub region: Option<CropRegion>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TapParams {
    #[schemars(description = "X coordinate to tap")]
    #[serde(deserialize_with = "deserialize_lenient_i32")]
    pub x: i32,

    #[schemars(description = "Y coordinate to tap")]
    #[serde(deserialize_with = "deserialize_lenient_i32")]
    pub y: i32,

    #[schemars(description = "Device serial number (optional, auto-selects if only one device)")]
    #[serde(default)]
    pub device: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SwipeParams {
    #[schemars(description = "Starting X coordinate")]
    #[serde(deserialize_with = "deserialize_lenient_i32")]
    pub start_x: i32,

    #[schemars(description = "Starting Y coordinate")]
    #[serde(deserialize_with = "deserialize_lenient_i32")]
    pub start_y: i32,

    #[schemars(description = "Ending X coordinate")]
    #[serde(deserialize_with = "deserialize_lenient_i32")]
    pub end_x: i32,

    #[schemars(description = "Ending Y coordinate")]
    #[serde(deserialize_with = "deserialize_lenient_i32")]
    pub end_y: i32,

    #[schemars(description = "Swipe duration in milliseconds (optional)")]
    #[serde(default, deserialize_with = "deserialize_lenient_u32_opt")]
    pub duration_ms: Option<u32>,

    #[schemars(description = "Device serial number (optional, auto-selects if only one device)")]
    #[serde(default)]
    pub device: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct InputTextParams {
    #[schemars(description = "Text to type on the device (requires focus on a text field)")]
    pub text: String,

    #[schemars(description = "Device serial number (optional, auto-selects if only one device)")]
    #[serde(default)]
    pub device: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct KeyeventParams {
    #[schemars(description = "Key to send (e.g., BACK, HOME, ENTER, or numeric keycode)")]
    pub key: String,

    #[schemars(description = "Device serial number (optional, auto-selects if only one device)")]
    #[serde(default)]
    pub device: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ShellParams {
    #[schemars(description = "Shell command to execute on the device")]
    pub command: String,

    #[schemars(description = "Device serial number (optional, auto-selects if only one device)")]
    #[serde(default)]
    pub device: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct UiDumpParams {
    #[schemars(description = "Device serial number (optional, auto-selects if only one device)")]
    #[serde(default)]
    pub device: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FindElementParams {
    #[schemars(description = "Text or content description to search for (substring match)")]
    #[serde(default)]
    pub text: Option<String>,

    #[schemars(description = "Resource ID to search for (substring match)")]
    #[serde(default)]
    pub resource_id: Option<String>,

    #[schemars(description = "Class name to filter by (substring match)")]
    #[serde(default)]
    pub class: Option<String>,

    #[schemars(description = "Device serial number (optional, auto-selects if only one device)")]
    #[serde(default)]
    pub device: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TapElementParams {
    #[schemars(description = "Text or content description to search for (substring match)")]
    #[serde(default)]
    pub text: Option<String>,

    #[schemars(description = "Resource ID to search for (substring match)")]
    #[serde(default)]
    pub resource_id: Option<String>,

    #[schemars(description = "Class name to filter by (substring match)")]
    #[serde(default)]
    pub class: Option<String>,

    #[schemars(description = "Device serial number (optional, auto-selects if only one device)")]
    #[serde(default)]
    pub device: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetActivityParams {
    #[schemars(description = "Device serial number (optional, auto-selects if only one device)")]
    #[serde(default)]
    pub device: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WaitForActivityParams {
    #[schemars(description = "Activity name or pattern to wait for")]
    pub activity: String,

    #[schemars(description = "Timeout in milliseconds (default: 10000)")]
    #[serde(default, deserialize_with = "deserialize_lenient_u64_opt")]
    pub timeout_ms: Option<u64>,

    #[schemars(description = "Device serial number (optional, auto-selects if only one device)")]
    #[serde(default)]
    pub device: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn tap_params_from_string_coords() {
        let json = r#"{"x": "631", "y": "800"}"#;
        let params: TapParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.x, 631);
        assert_eq!(params.y, 800);
    }

    #[test]
    fn tap_params_from_native_ints() {
        let json = r#"{"x": 631, "y": 800}"#;
        let params: TapParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.x, 631);
        assert_eq!(params.y, 800);
    }

    #[test]
    fn swipe_params_strings() {
        let json = r#"{"start_x": "100", "start_y": "200", "end_x": "300", "end_y": "400"}"#;
        let params: SwipeParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.start_x, 100);
        assert_eq!(params.start_y, 200);
        assert_eq!(params.end_x, 300);
        assert_eq!(params.end_y, 400);
        assert_eq!(params.duration_ms, None);
    }

    #[test]
    fn swipe_params_with_duration_string() {
        let json = r#"{"start_x": 100, "start_y": 200, "end_x": 300, "end_y": 400, "duration_ms": "500"}"#;
        let params: SwipeParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.duration_ms, Some(500));
    }

    #[test]
    fn optional_u8_from_string() {
        let json = r#"{"quality": "80"}"#;
        // Parse into a helper struct to test the field in isolation
        #[derive(Deserialize)]
        struct Q {
            #[serde(default, deserialize_with = "deserialize_lenient_u8_opt")]
            quality: Option<u8>,
        }
        let q: Q = serde_json::from_str(json).unwrap();
        assert_eq!(q.quality, Some(80));
    }

    #[test]
    fn optional_u32_null() {
        let json = r#"{"max_width": null}"#;
        #[derive(Deserialize)]
        struct W {
            #[serde(default, deserialize_with = "deserialize_lenient_u32_opt")]
            max_width: Option<u32>,
        }
        let w: W = serde_json::from_str(json).unwrap();
        assert_eq!(w.max_width, None);
    }

    #[test]
    fn optional_u64_from_string() {
        let json = r#"{"activity": "com.example/.Main", "timeout_ms": "5000"}"#;
        let params: WaitForActivityParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.timeout_ms, Some(5000));
    }

    #[test]
    fn crop_region_from_strings() {
        let json = r#"{"x": "10", "y": "20", "width": "100", "height": "200"}"#;
        let region: CropRegion = serde_json::from_str(json).unwrap();
        assert_eq!(region.x, 10);
        assert_eq!(region.y, 20);
        assert_eq!(region.width, 100);
        assert_eq!(region.height, 200);
    }

    #[test]
    fn invalid_string_rejected() {
        let json = r#"{"x": "abc", "y": "800"}"#;
        let result = serde_json::from_str::<TapParams>(json);
        assert!(result.is_err());
    }

    #[test]
    fn negative_i32_from_string() {
        let json = r#"{"x": "-50", "y": "100"}"#;
        let params: TapParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.x, -50);
        assert_eq!(params.y, 100);
    }
}
