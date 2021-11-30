use crate::{Config, ContainerConfig, Server};
use derive_builder::Builder;
use dockertest::{waitfor, PullPolicy, Source};
use std::collections::HashMap;

const IMAGE: &str = "ghcr.io/navikt/mock-oauth2-server";
const PORT: u32 = 8080;
const LOG_MSG: &str = "started server on address";
const SOURCE: Source = Source::DockerHub(PullPolicy::IfNotPresent);

/// Configuration for creating a mock OAuth (OIDC) server.
///
/// By default the OAuth server listens on port 8080 for HTTP requests. This
/// is exposed on the container by default, but the exposed port can be
/// controlled by setting the `port` field.
///
/// See the [Github](https://github.com/navikt/mock-oauth2-server) repo for more
/// information on the arguments and environment variables that can be used to
/// configure the server.
#[derive(Clone, Default, Builder)]
#[builder(default)]
pub struct OIDCServerConfig {
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
    #[builder(default = "String::from(\"0.3.5\")")]
    pub version: String,
}

impl OIDCServerConfig {
    pub fn builder() -> OIDCServerConfigBuilder {
        OIDCServerConfigBuilder::default()
    }
}

impl Config for OIDCServerConfig {
    fn into_composition(self) -> dockertest::Composition {
        let ports = vec![(PORT, self.port)];

        let timeout = self.timeout;
        let wait = Box::new(waitfor::MessageWait {
            message: LOG_MSG.into(),
            source: waitfor::MessageSource::Stdout,
            timeout,
        });

        ContainerConfig {
            args: self.args,
            env: self.env,
            handle: self.handle,
            name: IMAGE.into(),
            source: SOURCE,
            version: self.version,
            ports: Some(ports),
            wait: Some(wait),
            bind_mounts: HashMap::new(),
        }
        .into()
    }

    fn handle(&self) -> &str {
        self.handle.as_str()
    }
}

/// A running instance of a mock OAuth server.
///
/// The server URL which is accessible from the local host can be found in
/// `local_address`. Other running containers which need access to this server
/// should use the `address` field instead.
pub struct OIDCServer {
    pub external_port: u32,
    pub internal_port: u32,
    pub ip: String,
}

impl OIDCServer {
    fn format_address(&self, host: &str, port: u32) -> String {
        format!("{}:{}", host, port)
    }

    fn format_url(&self, host: &str, port: u32) -> String {
        format!("http://{}", self.format_address(host, port))
    }

    /// The external address in the form of localhost:{port}
    pub fn external_address(&self) -> String {
        self.format_address("localhost", self.external_port)
    }

    /// The external HTTP address
    pub fn external_url(&self) -> String {
        self.format_url("localhost", self.external_port)
    }

    /// The container internal address in the form of {ip}:{port}
    pub fn internal_address(&self) -> String {
        self.format_address(self.ip.as_str(), self.internal_port)
    }

    /// The internal HTTP address
    pub fn internal_url(&self) -> String {
        self.format_url(self.ip.as_str(), self.internal_port)
    }
}

impl Server for OIDCServer {
    type Config = OIDCServerConfig;

    fn new(config: &Self::Config, container: &dockertest::RunningContainer) -> Self {
        OIDCServer {
            external_port: config.port,
            internal_port: PORT,
            ip: container.ip().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{OIDCServer, OIDCServerConfig};
    use crate::Test;
    use test_env_log::test;

    const PORT: u32 = 9080;

    #[test]
    fn test_oidc() {
        let config = OIDCServerConfig::builder().port(PORT).build().unwrap();
        let mut test = Test::new();
        test.register(config);

        test.run(|instance| async move {
            let server: OIDCServer = instance.server();

            let client = reqwest::Client::new();
            let resp = client
                .get(format!(
                    "{}/default/.well-known/openid-configuration",
                    server.external_url()
                ))
                .send()
                .await;
            assert!(resp.is_ok());
            assert_eq!(resp.unwrap().status(), 200);
        });
    }
}
