//! Helpers shared by independently compiled integration-test binaries.
// Each binary uses a different subset of the shared helpers and fixtures.
#![allow(dead_code)]

mod coverage;
pub mod fixtures;
pub mod http;
