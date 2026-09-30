//! HCS Linux — HCS Chat library target.
//!
//! Splitting lib + bin lets the GUI be integration-tested (`tests/gui_tests.rs`)
//! while the CLI in `main.rs` stays the same binary as before.

pub mod gui;
