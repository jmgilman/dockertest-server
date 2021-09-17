/// Contains [Servers][Server] for Hashicorp products.
pub mod consul;
pub mod vault;

pub use consul::{ConsulServer, ConsulServerConfig};
pub use vault::{VaultServer, VaultServerConfig};
