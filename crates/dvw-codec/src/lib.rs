//! Lossless read and write of DataVolley `.dvw` and VolleyStation `.vsm` files.
//!
//! The format is undocumented. The golden corpus in `testdata/` is the
//! specification and openvolley is the oracle; every quirk found goes in
//! `docs/FORMAT-NOTES.md`.

pub mod date;
pub mod dvw;
pub mod encoding;
pub mod scout;
pub mod sections;
pub mod text;
pub mod vsm;

pub use date::Date;
pub use dvw::Dvw;
pub use encoding::TextEncoding;
pub use sections::RawFile;
pub use vsm::RawVsm;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid .vsm JSON: {0}")]
    VsmJson(#[from] serde_json::Error),
    #[error("unknown code page {0:?} in [3MATCH]; pass an encoding explicitly")]
    UnknownCodepage(String),
    #[error("[{section}] line {line} is not valid {encoding}")]
    Undecodable {
        encoding: &'static str,
        section: String,
        line: usize,
    },
    #[error("[{section}] line {line} has text that {encoding} cannot represent")]
    Unencodable {
        encoding: &'static str,
        section: String,
        line: usize,
    },
}
