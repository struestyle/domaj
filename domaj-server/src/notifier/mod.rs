//! Notification module for Domaj Server
//!
//! Handles sending update reports via email.

mod email;

pub use email::send_update_report;
