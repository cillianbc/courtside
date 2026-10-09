//! Text encodings of `.dvw` files.
//!
//! `[3MATCH]` field 9 holds a Windows code page number (`1252` in every
//! corpus file so far). Decoding must be reversible: a file only parses if
//! re-encoding the decoded text gives back the same bytes.

use encoding_rs::Encoding;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextEncoding(&'static Encoding);

impl TextEncoding {
    pub const WINDOWS_1252: TextEncoding = TextEncoding(encoding_rs::WINDOWS_1252);
    pub const UTF_8: TextEncoding = TextEncoding(encoding_rs::UTF_8);

    /// From a Windows code page number as written in `[3MATCH]`.
    pub fn from_codepage(codepage: &str) -> Option<Self> {
        let encoding = match codepage.trim() {
            "1250" => encoding_rs::WINDOWS_1250,
            "1251" => encoding_rs::WINDOWS_1251,
            "1252" => encoding_rs::WINDOWS_1252,
            "1253" => encoding_rs::WINDOWS_1253,
            "1254" => encoding_rs::WINDOWS_1254,
            "1255" => encoding_rs::WINDOWS_1255,
            "1256" => encoding_rs::WINDOWS_1256,
            "1257" => encoding_rs::WINDOWS_1257,
            "1258" => encoding_rs::WINDOWS_1258,
            "874" => encoding_rs::WINDOWS_874,
            "932" => encoding_rs::SHIFT_JIS,
            "936" => encoding_rs::GBK,
            "949" => encoding_rs::EUC_KR,
            "950" => encoding_rs::BIG5,
            "28592" => encoding_rs::ISO_8859_2,
            "28605" => encoding_rs::ISO_8859_15,
            "65001" => encoding_rs::UTF_8,
            _ => return None,
        };
        Some(TextEncoding(encoding))
    }

    /// From a WHATWG label such as `windows-1250` or `iso-8859-2`.
    pub fn from_label(label: &str) -> Option<Self> {
        Encoding::for_label(label.as_bytes()).map(TextEncoding)
    }

    pub fn name(self) -> &'static str {
        self.0.name()
    }

    /// Decodes `bytes`; `None` unless encoding the result reproduces them.
    pub fn decode(self, bytes: &[u8]) -> Option<String> {
        let text = self
            .0
            .decode_without_bom_handling_and_without_replacement(bytes)?
            .into_owned();
        (self.encode(&text)? == bytes).then_some(text)
    }

    /// Encodes `text`; `None` if the encoding cannot represent it.
    pub fn encode(self, text: &str) -> Option<Vec<u8>> {
        let (bytes, _, unmappable) = self.0.encode(text);
        (!unmappable).then(|| bytes.into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_1252_round_trips_every_byte() {
        let bytes: Vec<u8> = (0..=255).collect();
        let text = TextEncoding::WINDOWS_1252.decode(&bytes).unwrap();
        assert_eq!(TextEncoding::WINDOWS_1252.encode(&text).unwrap(), bytes);
    }

    #[test]
    fn rejects_invalid_utf8() {
        assert!(TextEncoding::UTF_8.decode(&[0x41, 0xED, 0x6E]).is_none());
    }

    #[test]
    fn rejects_unrepresentable_text() {
        assert!(TextEncoding::WINDOWS_1252.encode("Łódź").is_none());
        assert!(
            TextEncoding::from_codepage("1250")
                .unwrap()
                .encode("Łódź")
                .is_some()
        );
    }

    #[test]
    fn unknown_codepage() {
        assert_eq!(TextEncoding::from_codepage("9999"), None);
    }
}
