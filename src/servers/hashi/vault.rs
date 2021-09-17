use crate::common::rand_string;
use crate::{Config, ContainerConfig, Server};
use derive_builder::Builder;
use dockertest::{PullPolicy, Source};
use std::collections::HashMap;

const IMAGE: &str = "vault";
const PORT: u32 = 8200;
const LOG_MSG: &str = "Development mode should NOT be used in production installations!";
const SOURCE: Source = Source::DockerHub(PullPolicy::IfNotPresent);

/// Configuration for creating a Hashicorp Vault server.
///
/// A token with root permisions will automatically be generated using the
/// `token` field. If it's omitted the token will automatically be generated.
///
/// By default the Vault server listens on port 8200 for HTTP requests. This
/// is exposed on the container by default, but the exposed port can be
/// controlled by setting the `port` field.
///
/// See the [Dockerhub](https://hub.docker.com/_/vault) page for more
/// information on the arguments and environment variables that can be used to
/// configure the server.
#[derive(Clone, Default, Builder)]
#[builder(default)]
pub struct VaultServerConfig {
    #[builder(default = "Vec::new()")]
    pub args: Vec<String>,
    #[builder(default = "HashMap::new()")]
    pub env: HashMap<String, String>,
    #[builder(default = "crate::server::new_handle(IMAGE)")]
    pub handle: String,
    #[builder(default = "8200")]
    pub port: u32,
    #[builder(default = "15")]
    pub timeout: u16,
    #[builder(default = "rand_string(16)")]
    pub token: String,
    #[builder(default = "String::from(\"latest\")")]
    pub version: String,
}

impl VaultServerConfig {
    pub fn builder() -> VaultServerConfigBuilder {
        VaultServerConfigBuilder::default()
    }
}

impl Config for VaultServerConfig {
    fn into_composition(self) -> dockertest::Composition {
        let ports = vec![(PORT, self.port)];
        let mut env = self.env.clone();
        env.insert(String::from("VAULT_DEV_ROOT_TOKEN_ID"), self.token.clone());

        ContainerConfig {
            args: self.args,
            env,
            handle: self.handle,
            name: IMAGE.into(),
            source: SOURCE,
            timeout: self.timeout,
            version: self.version,
            ports: Some(ports),
            wait_msg: Some(LOG_MSG.into()),
        }
        .into()
    }

    fn handle(&self) -> &str {
        self.handle.as_str()
    }
}

/// A running instane of a Vault server.
///
/// The `token` field contains the root Vault token for the server. The server
/// URL which is accessible from the local host can be found in `local_address`.
/// Other running containers which need access to this server should use the
/// `address` field instead.
pub struct VaultServer {
    pub address: String,
    pub external_port: u32,
    pub internal_port: u32,
    pub local_address: String,
    pub token: String,
}

impl Server for VaultServer {
    type Config = VaultServerConfig;

    fn new(config: &Self::Config, container: &dockertest::RunningContainer) -> Self {
        VaultServer {
            address: format!("http://{}:{}", container.ip(), PORT),
            external_port: config.port,
            internal_port: PORT,
            local_address: format!("http://localhost:{}", config.port),
            token: config.token.clone(),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::{VaultServer, VaultServerConfig};
    use crate::Test;

    #[test]
    fn test_vault() {
        let config = VaultServerConfig::builder()
            .port(9200)
            .version("1.8.2".into())
            .build()
            .unwrap();
        let mut test = Test::new();
        test.register(config);

        test.run(|instance| async move {
            let server: VaultServer = instance.server();

            let client = reqwest::Client::new();
            let resp = client
                .get(format!("{}/v1/auth/token/lookup", server.local_address))
                .header("X-Vault-Token", server.token)
                .send()
                .await;
            assert!(resp.is_ok());
            assert_eq!(resp.unwrap().status(), 200);
        });
    }
}
