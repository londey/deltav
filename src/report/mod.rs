//! Report generation module.
//!
//! Handles generating weekly reports in various formats.

pub mod data;
pub mod markdown;
pub mod html;

pub use data::ReportData;
