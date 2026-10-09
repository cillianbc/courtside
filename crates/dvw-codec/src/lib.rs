//! Lossless read and write of DataVolley `.dvw` and VolleyStation `.vsm` files.
//!
//! The format is undocumented. The golden corpus in `testdata/` is the
//! specification and openvolley is the oracle; every quirk found goes in
//! `docs/FORMAT-NOTES.md`.

#[derive(Debug, thiserror::Error)]
pub enum Error {}
