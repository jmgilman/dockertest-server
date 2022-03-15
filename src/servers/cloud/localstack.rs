use crate::{Config, ContainerConfig, Server};
use derive_builder::Builder;
use dockertest::{waitfor, PullPolicy, Source};
use std::collections::HashMap;

const IMAGE: &str = "localstack/localstack";
const PORT: u32 = 4566;
const LOG_MSG: &str = "Ready.";
const SOURCE: Source = Source::DockerHub(PullPolicy::IfNotPresent);

/// Configuration for creating a LocalStack server
///
/// By default the LocalStack server listens on port 4566 for requests. This
/// is exposed on the container by default, but the exposed port can be
/// controlled by setting the `port` field.
///
/// See the [DockerHub](https://hub.docker.com/localstack/localstack) repo for
/// more information on the arguments and environment variables that can be
/// used to configure the server.
#[derive(Clone, Default, Builder)]
#[builder(default)]
pub struct LocalStackServerConfig {
    #[builder(default = "Vec::new()")]
    pub args: Vec<String>,
    #[builder(default = "HashMap::new()")]
    pub env: HashMap<String, String>,
    #[builder(default = "crate::server::new_handle(IMAGE)")]
    pub handle: String,
    #[builder(default = "PORT")]
    pub port: u32,
    #[builder(default = "15")]
    pub timeout: u16,
    #[builder(default = "String::from(\"latest\")")]
    pub version: String,
}

impl LocalStackServerConfig {
    pub fn builder() -> LocalStackServerConfigBuilder {
        LocalStackServerConfigBuilder::default()
    }
}

impl Config for LocalStackServerConfig {
    fn into_composition(self) -> dockertest::Composition {
        let ports = vec![(PORT, self.port)];

        let wait = Box::new(waitfor::MessageWait {
            message: LOG_MSG.into(),
            source: waitfor::MessageSource::Stdout,
            timeout: self.timeout,
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

/// A running instance of a LocalStack server.
pub struct LocalStackServer {
    pub external_port: u32,
    pub internal_port: u32,
    pub ip: String,
}

impl LocalStackServer {
    fn format_address(&self, host: &str, port: u32) -> String {
        format!("{}:{}", host, port)
    }

    fn format_url(&self, host: &str, port: u32) -> String {
        format!("http://{}", self.format_address(host, port))
    }

    /// The external HTTP address
    pub fn external_url(&self) -> String {
        self.format_url("localhost", self.external_port)
    }

    /// The internal HTTP address
    pub fn internal_url(&self) -> String {
        self.format_url(self.ip.as_str(), self.internal_port)
    }

    /// The external address in the form of localhost:{port}
    pub fn external_address(&self) -> String {
        self.format_address("localhost", self.external_port)
    }

    /// The container internal address in the form of {ip}:{port}
    pub fn internal_address(&self) -> String {
        self.format_address(self.ip.as_str(), self.internal_port)
    }
}

impl Server for LocalStackServer {
    type Config = LocalStackServerConfig;

    fn new(config: &Self::Config, container: &dockertest::RunningContainer) -> Self {
        LocalStackServer {
            external_port: config.port,
            internal_port: PORT,
            ip: container.ip().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LocalStackServer, LocalStackServerConfig};
    use crate::Test;
    use std::collections::HashMap;
    use test_env_log::test;

    #[test]
    fn test_local_stack() {
        let env: HashMap<_, _> = vec![("SERVICES".to_string(), "iam,sts".to_string())]
            .into_iter()
            .collect();

        let config = LocalStackServerConfig::builder().env(env).build().unwrap();
        let mut test = Test::new();
        test.register(config);

        test.run(|instance| async move {
            let server: LocalStackServer = instance.server();
            let client = reqwest::Client::new();
            let resp = client
                .get(format!("{}/health", server.external_url()))
                .send()
                .await;
            assert!(resp.is_ok());
            assert_eq!(resp.unwrap().status(), 200);
        });
    }
}
