/// Contains ready-made [Servers][crate::Server] which can be used in tests.
#[cfg(feature = "auth")]
pub mod auth;
pub mod database;
#[cfg(feature = "hashi")]
pub mod hashi;
