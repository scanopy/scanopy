//! The app's single live-update stream: discovery progress, daemon check-ins and topology changes
//! over one SSE connection, so a browser tab holds one of the six HTTP/1.1 connections it gets per
//! host rather than one per feature.

pub mod handlers;
pub mod types;
