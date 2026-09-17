use std::io::Read;

use flate2::read::GzDecoder;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::error::{AnalyzerError, Result};

const MAX_DECOMPRESSED_BYTES: usize = 256 * 1024 * 1024;

pub fn decode_events(payload: &[u8]) -> Result<Vec<Value>> {
    let decoded = if payload.starts_with(&[0x1f, 0x8b]) {
        let mut decoder = GzDecoder::new(payload);
        let mut output = Vec::new();
        decoder
            .by_ref()
            .take((MAX_DECOMPRESSED_BYTES + 1) as u64)
            .read_to_end(&mut output)?;
        if output.len() > MAX_DECOMPRESSED_BYTES {
            return Err(AnalyzerError::InvalidTelemetry(
                "decompressed telemetry exceeds 256 MiB".to_owned(),
            ));
        }
        output
    } else {
        if payload.len() > MAX_DECOMPRESSED_BYTES {
            return Err(AnalyzerError::InvalidTelemetry(
                "telemetry exceeds 256 MiB".to_owned(),
            ));
        }
        payload.to_vec()
    };

    let value: Value = serde_json::from_slice(&decoded)?;
    let events = value.as_array().ok_or_else(|| {
        AnalyzerError::InvalidTelemetry("telemetry root must be an array".to_owned())
    })?;
    if events.iter().any(|event| !event.is_object()) {
        return Err(AnalyzerError::InvalidTelemetry(
            "every telemetry event must be an object".to_owned(),
        ));
    }
    Ok(events.clone())
}

#[must_use]
pub fn payload_sha256(payload: &[u8]) -> String {
    hex::encode(Sha256::digest(payload))
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use flate2::{Compression, write::GzEncoder};

    use super::decode_events;

    #[test]
    fn decodes_plain_and_gzip_arrays() {
        let plain = br#"[{"_T":"LogMatchStart"}]"#;
        assert_eq!(decode_events(plain).expect("plain JSON").len(), 1);

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(plain).expect("write gzip");
        let compressed = encoder.finish().expect("finish gzip");
        assert_eq!(decode_events(&compressed).expect("gzip JSON").len(), 1);
    }
}
