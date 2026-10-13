#![forbid(unsafe_code)]

//! Read-only observability dashboard (axum + a static Svelte SPA).
//!
//! The dashboard serves a single-page app (committed under `dist/`) that renders
//! the latest [`ServerSnapshot`] published by the application driver through a
//! [`SnapshotPublisher`]. The SPA seeds itself from `GET /api/snapshot` and then
//! streams live updates over `GET /events` (Server-Sent Events), falling back to
//! polling the JSON endpoint when `EventSource` is unavailable.
//!
//! It is read-only by construction: every route is a `GET` (the JSON endpoint, the
//! SSE stream, and the static file service are all reads), a method-guard layer
//! rejects anything else with `405`. The application chooses the listener address;
//! the default IPv4 wildcard serves the dashboard on all configured interfaces.
//!
//! [`ServerSnapshot`]: ferrumc_observability::ServerSnapshot
//! [`SnapshotPublisher`]: ferrumc_observability::SnapshotPublisher

mod api;
mod prometheus;
mod server;

use std::fmt;
use std::net::SocketAddr;

use ferrumc_observability::SnapshotPublisher;

pub use server::router;

/// An error from starting the read-only dashboard server.
///
/// A small dependency-free enum (no `anyhow`): each variant classifies the
/// failure so the caller can log it precisely.
#[derive(Debug)]
pub enum DashboardError {
    /// Binding the listener or running the accept loop failed.
    Bind(std::io::Error),
}

impl fmt::Display for DashboardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bind(err) => write!(f, "dashboard listener failed: {err}"),
        }
    }
}

impl std::error::Error for DashboardError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Bind(err) => Some(err),
        }
    }
}

impl From<std::io::Error> for DashboardError {
    fn from(err: std::io::Error) -> Self {
        Self::Bind(err)
    }
}

/// Binds the read-only dashboard on the application-selected address.
///
/// Wildcard, loopback and specific interface addresses are supported. The
/// listener preserves the requested port, including an OS-assigned port zero.
/// `snapshots` is a read handle; all HTTP routes retain their read-only guard.
///
/// # Errors
///
/// Returns [`DashboardError::Bind`] if the address cannot be bound or the accept
/// loop fails.
pub async fn run(bind: SocketAddr, snapshots: SnapshotPublisher) -> Result<(), DashboardError> {
    let listener = tokio::net::TcpListener::bind(bind).await?;
    axum::serve(listener, router(snapshots)).await?;
    Ok(())
}
