//! Places: where records rest on one machine.
//!
//! The pure `loops` crate reads records and never touches a disk, a clock or
//! a key. This crate is the first thing around it that does:
//!
//! * [`mod@line`] reads and writes the signed stored line of `protocol/record.md`,
//!   and turns a line into the record the core's readers take;
//! * [`dir`] keeps lines in a directory of immutable segments;
//! * [`fill`] fills in what the single-record emit leaves out: `of`, the
//!   anchor and `about`;
//! * [`key`] holds a signing key and makes salts;
//! * [`clock`] writes the canonical instant for now.
//!
//! Standing arrives with step 3. Until then every signed line whose bytes
//! check is held, and nothing here decides who may write.
pub mod clock;
pub mod dir;
pub mod fill;
pub mod key;
pub mod line;
