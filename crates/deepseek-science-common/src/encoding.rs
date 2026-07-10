//! Strict bounded byte-to-text inspection for supported Unicode encodings.
//!
//! This module is pure and in-memory. It recognizes a deliberately small BOM
//! set, decodes without replacement characters, and performs no file IO or
//! table interpretation.

use std::error::Error;
use std::fmt;
use std::str;

const UTF8_BOM: &[u8; 3] = b"\xEF\xBB\xBF";
const UTF16_LE_BOM: &[u8; 2] = b"\xFF\xFE";
const UTF16_BE_BOM: &[u8; 2] = b"\xFE\xFF";
const UTF32_LE_BOM: &[u8; 4] = b"\xFF\xFE\x00\x00";
const UTF32_BE_BOM: &[u8; 4] = b"\x00\x00\xFE\xFF";

/// Fixed maximum byte length accepted by v0.4 text inspection.
pub const MAX_INSPECTION_BYTES: usize = 16 * 1024 * 1024;

/// Byte order mark observed at the start of the original input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ByteOrderMark {
    /// No BOM was present.
    None,
    /// UTF-8 BOM (`EF BB BF`).
    Utf8,
    /// UTF-16 little-endian BOM (`FF FE`).
    Utf16Le,
    /// UTF-16 big-endian BOM (`FE FF`).
    Utf16Be,
}

/// Encoding used for strict text decoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextEncoding {
    /// UTF-8, with or without its BOM.
    Utf8,
    /// BOM-required UTF-16 little-endian.
    Utf16Le,
    /// BOM-required UTF-16 big-endian.
    Utf16Be,
}

/// Successful bounded text-encoding inspection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncodingInspection {
    /// Strictly decoded owned text with one supported BOM removed.
    pub text: String,
    /// Encoding used to decode the input.
    pub encoding: TextEncoding,
    /// BOM observed in the original input.
    pub bom: ByteOrderMark,
    /// Original input length before BOM removal or decoding.
    pub original_byte_len: usize,
}

/// Errors raised by bounded strict text-encoding inspection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EncodingInspectionError {
    /// Input exceeded the fixed inspection byte limit.
    InspectionLimitExceeded {
        /// Original input byte length.
        actual: usize,
        /// Maximum accepted byte length.
        maximum: usize,
    },
    /// Input contained binary evidence unsupported by the text boundary.
    UnsupportedBinaryInput {
        /// Zero-based offset in the original input.
        byte_offset: usize,
    },
    /// BOM-free or conflicting input could not be assigned a supported encoding.
    UnsupportedOrAmbiguousEncoding {
        /// Zero-based offset in the original input where ambiguity begins.
        byte_offset: usize,
    },
    /// A UTF-8 BOM selected UTF-8, but its payload was invalid.
    InvalidUtf8 {
        /// Zero-based offset in the original input of the invalid sequence.
        byte_offset: usize,
    },
    /// A UTF-16 BOM selected UTF-16, but its payload was invalid.
    InvalidUtf16 {
        /// Zero-based offset in the original input of the invalid byte or code unit.
        byte_offset: usize,
    },
}

impl fmt::Display for EncodingInspectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InspectionLimitExceeded { actual, maximum } => write!(
                formatter,
                "input has {actual} bytes, exceeding inspection limit of {maximum} bytes"
            ),
            Self::UnsupportedBinaryInput { byte_offset } => {
                write!(
                    formatter,
                    "unsupported binary input at byte offset {byte_offset}"
                )
            }
            Self::UnsupportedOrAmbiguousEncoding { byte_offset } => write!(
                formatter,
                "unsupported or ambiguous encoding at byte offset {byte_offset}"
            ),
            Self::InvalidUtf8 { byte_offset } => {
                write!(formatter, "invalid UTF-8 at byte offset {byte_offset}")
            }
            Self::InvalidUtf16 { byte_offset } => {
                write!(formatter, "invalid UTF-16 at byte offset {byte_offset}")
            }
        }
    }
}

impl Error for EncodingInspectionError {}

/// Inspects and strictly decodes one bounded in-memory byte input.
///
/// UTF-8 is accepted with or without one BOM. UTF-16LE and UTF-16BE require
/// their BOM. UTF-32, repeated or conflicting BOMs, NUL text, BOM-free invalid
/// UTF-8, and inputs larger than [`MAX_INSPECTION_BYTES`] are rejected.
pub fn inspect_text_encoding(input: &[u8]) -> Result<EncodingInspection, EncodingInspectionError> {
    inspect_text_encoding_with_limit(input, MAX_INSPECTION_BYTES)
}

fn inspect_text_encoding_with_limit(
    input: &[u8],
    maximum: usize,
) -> Result<EncodingInspection, EncodingInspectionError> {
    if input.len() > maximum {
        return Err(EncodingInspectionError::InspectionLimitExceeded {
            actual: input.len(),
            maximum,
        });
    }

    if input.starts_with(UTF32_LE_BOM) || input.starts_with(UTF32_BE_BOM) {
        return Err(EncodingInspectionError::UnsupportedBinaryInput { byte_offset: 0 });
    }

    if input.starts_with(UTF8_BOM) {
        return decode_utf8_with_bom(input);
    }
    if input.starts_with(UTF16_LE_BOM) {
        return decode_utf16(
            input,
            ByteOrderMark::Utf16Le,
            TextEncoding::Utf16Le,
            u16::from_le_bytes,
        );
    }
    if input.starts_with(UTF16_BE_BOM) {
        return decode_utf16(
            input,
            ByteOrderMark::Utf16Be,
            TextEncoding::Utf16Be,
            u16::from_be_bytes,
        );
    }

    decode_utf8_without_bom(input)
}

fn decode_utf8_with_bom(input: &[u8]) -> Result<EncodingInspection, EncodingInspectionError> {
    let payload = &input[UTF8_BOM.len()..];
    reject_nested_bom(payload, UTF8_BOM.len())?;
    let text = str::from_utf8(payload).map_err(|error| EncodingInspectionError::InvalidUtf8 {
        byte_offset: UTF8_BOM.len() + error.valid_up_to(),
    })?;
    reject_utf8_nul(text, UTF8_BOM.len())?;

    Ok(EncodingInspection {
        text: text.to_owned(),
        encoding: TextEncoding::Utf8,
        bom: ByteOrderMark::Utf8,
        original_byte_len: input.len(),
    })
}

fn decode_utf8_without_bom(input: &[u8]) -> Result<EncodingInspection, EncodingInspectionError> {
    if let Some(byte_offset) = input.iter().position(|byte| *byte == 0) {
        return Err(EncodingInspectionError::UnsupportedBinaryInput { byte_offset });
    }

    let text = str::from_utf8(input).map_err(|error| {
        EncodingInspectionError::UnsupportedOrAmbiguousEncoding {
            byte_offset: error.valid_up_to(),
        }
    })?;
    reject_utf8_nul(text, 0)?;

    Ok(EncodingInspection {
        text: text.to_owned(),
        encoding: TextEncoding::Utf8,
        bom: ByteOrderMark::None,
        original_byte_len: input.len(),
    })
}

fn decode_utf16(
    input: &[u8],
    bom: ByteOrderMark,
    encoding: TextEncoding,
    decode_unit: fn([u8; 2]) -> u16,
) -> Result<EncodingInspection, EncodingInspectionError> {
    let payload = &input[UTF16_LE_BOM.len()..];
    reject_nested_bom(payload, UTF16_LE_BOM.len())?;

    if payload.len() % 2 != 0 {
        return Err(EncodingInspectionError::InvalidUtf16 {
            byte_offset: input.len() - 1,
        });
    }

    let units = payload
        .chunks_exact(2)
        .map(|bytes| decode_unit([bytes[0], bytes[1]]));
    let mut text = String::with_capacity(payload.len() / 2);
    let mut code_unit_index = 0;

    for decoded in char::decode_utf16(units) {
        let byte_offset = UTF16_LE_BOM.len() + code_unit_index * 2;
        let character =
            decoded.map_err(|_| EncodingInspectionError::InvalidUtf16 { byte_offset })?;
        if character == '\0' {
            return Err(EncodingInspectionError::UnsupportedBinaryInput { byte_offset });
        }
        code_unit_index += character.len_utf16();
        text.push(character);
    }

    Ok(EncodingInspection {
        text,
        encoding,
        bom,
        original_byte_len: input.len(),
    })
}

fn reject_nested_bom(payload: &[u8], byte_offset: usize) -> Result<(), EncodingInspectionError> {
    if payload.starts_with(UTF32_LE_BOM)
        || payload.starts_with(UTF32_BE_BOM)
        || payload.starts_with(UTF8_BOM)
        || payload.starts_with(UTF16_LE_BOM)
        || payload.starts_with(UTF16_BE_BOM)
    {
        return Err(EncodingInspectionError::UnsupportedOrAmbiguousEncoding { byte_offset });
    }

    Ok(())
}

fn reject_utf8_nul(text: &str, payload_offset: usize) -> Result<(), EncodingInspectionError> {
    if let Some((text_offset, _)) = text
        .char_indices()
        .find(|(_, character)| *character == '\0')
    {
        return Err(EncodingInspectionError::UnsupportedBinaryInput {
            byte_offset: payload_offset + text_offset,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        inspect_text_encoding, inspect_text_encoding_with_limit, ByteOrderMark,
        EncodingInspectionError, TextEncoding, MAX_INSPECTION_BYTES,
    };

    #[test]
    fn empty_input_is_utf8_without_bom() {
        let inspection = inspect_text_encoding(b"").expect("empty input should decode");

        assert_eq!(inspection.text, "");
        assert_eq!(inspection.encoding, TextEncoding::Utf8);
        assert_eq!(inspection.bom, ByteOrderMark::None);
        assert_eq!(inspection.original_byte_len, 0);
    }

    #[test]
    fn ascii_utf8_without_bom_is_preserved() {
        let inspection = inspect_text_encoding(b"laboratory").expect("ASCII should decode");

        assert_eq!(inspection.text, "laboratory");
        assert_eq!(inspection.encoding, TextEncoding::Utf8);
        assert_eq!(inspection.bom, ByteOrderMark::None);
    }

    #[test]
    fn multibyte_utf8_without_bom_is_preserved() {
        let input = "光谱 Δ".as_bytes();
        let inspection = inspect_text_encoding(input).expect("multibyte UTF-8 should decode");

        assert_eq!(inspection.text, "光谱 Δ");
        assert_eq!(inspection.original_byte_len, input.len());
    }

    #[test]
    fn utf8_bom_is_recognized_and_stripped_once() {
        let input = b"\xEF\xBB\xBFsample";
        let inspection = inspect_text_encoding(input).expect("UTF-8 BOM input should decode");

        assert_eq!(inspection.text, "sample");
        assert_eq!(inspection.encoding, TextEncoding::Utf8);
        assert_eq!(inspection.bom, ByteOrderMark::Utf8);
        assert_eq!(inspection.original_byte_len, input.len());
    }

    #[test]
    fn utf16le_bom_with_ascii_content_decodes() {
        let input = [0xFF, 0xFE, 0x41, 0x00, 0x42, 0x00];
        let inspection = inspect_text_encoding(&input).expect("UTF-16LE should decode");

        assert_eq!(inspection.text, "AB");
        assert_eq!(inspection.encoding, TextEncoding::Utf16Le);
        assert_eq!(inspection.bom, ByteOrderMark::Utf16Le);
    }

    #[test]
    fn utf16be_bom_with_ascii_content_decodes() {
        let input = [0xFE, 0xFF, 0x00, 0x41, 0x00, 0x42];
        let inspection = inspect_text_encoding(&input).expect("UTF-16BE should decode");

        assert_eq!(inspection.text, "AB");
        assert_eq!(inspection.encoding, TextEncoding::Utf16Be);
        assert_eq!(inspection.bom, ByteOrderMark::Utf16Be);
    }

    #[test]
    fn utf16le_valid_surrogate_pair_decodes() {
        let input = [0xFF, 0xFE, 0x3D, 0xD8, 0x00, 0xDE];
        let inspection = inspect_text_encoding(&input).expect("surrogate pair should decode");

        assert_eq!(inspection.text, "😀");
    }

    #[test]
    fn utf16be_valid_surrogate_pair_decodes() {
        let input = [0xFE, 0xFF, 0xD8, 0x3D, 0xDE, 0x00];
        let inspection = inspect_text_encoding(&input).expect("surrogate pair should decode");

        assert_eq!(inspection.text, "😀");
    }

    #[test]
    fn original_byte_length_is_preserved() {
        let input = [0xFF, 0xFE, 0x41, 0x00];
        let inspection = inspect_text_encoding(&input).expect("UTF-16LE should decode");

        assert_eq!(inspection.original_byte_len, 4);
    }

    #[test]
    fn newline_containing_text_is_unchanged() {
        let inspection = inspect_text_encoding(b"first\nsecond\r\n")
            .expect("newline-containing UTF-8 should decode");

        assert_eq!(inspection.text, "first\nsecond\r\n");
    }

    #[test]
    fn input_above_fixed_limit_is_rejected_before_decoding() {
        assert_eq!(MAX_INSPECTION_BYTES, 16 * 1024 * 1024);
        assert_eq!(
            inspect_text_encoding_with_limit(&[0xFF, 0xFF], 1),
            Err(EncodingInspectionError::InspectionLimitExceeded {
                actual: 2,
                maximum: 1,
            })
        );
    }

    #[test]
    fn utf32le_bom_is_rejected_before_utf16le_prefix_matching() {
        assert_eq!(
            inspect_text_encoding(&[0xFF, 0xFE, 0x00, 0x00]),
            Err(EncodingInspectionError::UnsupportedBinaryInput { byte_offset: 0 })
        );
    }

    #[test]
    fn utf32be_bom_is_rejected() {
        assert_eq!(
            inspect_text_encoding(&[0x00, 0x00, 0xFE, 0xFF]),
            Err(EncodingInspectionError::UnsupportedBinaryInput { byte_offset: 0 })
        );
    }

    #[test]
    fn repeated_utf8_bom_is_rejected() {
        assert_eq!(
            inspect_text_encoding(b"\xEF\xBB\xBF\xEF\xBB\xBFtext"),
            Err(EncodingInspectionError::UnsupportedOrAmbiguousEncoding { byte_offset: 3 })
        );
    }

    #[test]
    fn repeated_utf16le_bom_is_rejected() {
        assert_eq!(
            inspect_text_encoding(&[0xFF, 0xFE, 0xFF, 0xFE, 0x41, 0x00]),
            Err(EncodingInspectionError::UnsupportedOrAmbiguousEncoding { byte_offset: 2 })
        );
    }

    #[test]
    fn repeated_utf16be_bom_is_rejected() {
        assert_eq!(
            inspect_text_encoding(&[0xFE, 0xFF, 0xFE, 0xFF, 0x00, 0x41]),
            Err(EncodingInspectionError::UnsupportedOrAmbiguousEncoding { byte_offset: 2 })
        );
    }

    #[test]
    fn utf8_bom_with_invalid_payload_reports_original_offset() {
        assert_eq!(
            inspect_text_encoding(&[0xEF, 0xBB, 0xBF, b'A', 0xFF]),
            Err(EncodingInspectionError::InvalidUtf8 { byte_offset: 4 })
        );
    }

    #[test]
    fn bom_free_invalid_utf8_is_ambiguous_not_utf16() {
        assert_eq!(
            inspect_text_encoding(&[b'A', 0xFF]),
            Err(EncodingInspectionError::UnsupportedOrAmbiguousEncoding { byte_offset: 1 })
        );
    }

    #[test]
    fn utf16le_odd_payload_reports_unmatched_final_byte() {
        assert_eq!(
            inspect_text_encoding(&[0xFF, 0xFE, 0x41, 0x00, 0x42]),
            Err(EncodingInspectionError::InvalidUtf16 { byte_offset: 4 })
        );
    }

    #[test]
    fn utf16be_odd_payload_reports_unmatched_final_byte() {
        assert_eq!(
            inspect_text_encoding(&[0xFE, 0xFF, 0x00, 0x41, 0x42]),
            Err(EncodingInspectionError::InvalidUtf16 { byte_offset: 4 })
        );
    }

    #[test]
    fn utf16_unpaired_high_surrogate_reports_code_unit_offset() {
        assert_eq!(
            inspect_text_encoding(&[0xFF, 0xFE, 0x41, 0x00, 0x00, 0xD8]),
            Err(EncodingInspectionError::InvalidUtf16 { byte_offset: 4 })
        );
    }

    #[test]
    fn utf16_unpaired_low_surrogate_reports_code_unit_offset() {
        assert_eq!(
            inspect_text_encoding(&[0xFE, 0xFF, 0x00, 0x41, 0xDC, 0x00]),
            Err(EncodingInspectionError::InvalidUtf16 { byte_offset: 4 })
        );
    }

    #[test]
    fn bom_free_utf8_nul_is_rejected_as_binary() {
        assert_eq!(
            inspect_text_encoding(b"A\0B"),
            Err(EncodingInspectionError::UnsupportedBinaryInput { byte_offset: 1 })
        );
    }

    #[test]
    fn utf8_bom_payload_nul_is_rejected_as_binary() {
        assert_eq!(
            inspect_text_encoding(b"\xEF\xBB\xBFA\0"),
            Err(EncodingInspectionError::UnsupportedBinaryInput { byte_offset: 4 })
        );
    }

    #[test]
    fn utf16_decoded_nul_is_rejected_as_binary() {
        assert_eq!(
            inspect_text_encoding(&[0xFF, 0xFE, 0x41, 0x00, 0x00, 0x00]),
            Err(EncodingInspectionError::UnsupportedBinaryInput { byte_offset: 4 })
        );
    }

    #[test]
    fn conflicting_bom_sequence_is_rejected() {
        assert_eq!(
            inspect_text_encoding(&[0xEF, 0xBB, 0xBF, 0xFF, 0xFE]),
            Err(EncodingInspectionError::UnsupportedOrAmbiguousEncoding { byte_offset: 3 })
        );
    }

    #[test]
    fn truncated_bom_like_input_is_rejected_conservatively() {
        assert_eq!(
            inspect_text_encoding(&[0xEF, 0xBB]),
            Err(EncodingInspectionError::UnsupportedOrAmbiguousEncoding { byte_offset: 0 })
        );
    }
}
