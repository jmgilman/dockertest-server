use crate::common::rand_string;
use crate::{Config, ContainerConfig, Server};
use derive_builder::Builder;
use dockertest::{waitfor, PullPolicy, Source};
use std::collections::HashMap;

const IMAGE: &str = "hashicorp/counting-service";
const PORT: u32 = 9001;
const LOG_MSG: &str = "Serving at";
const SOURCE: Source = Source::DockerHub(PullPolicy::IfNotPresent);

/// Configuration for creating a Hashicorp Counting Server instance
///
/// By default the server listens on port 9001 for HTTP requests. This
/// is exposed on the container by default, but the exposed port can be
/// controlled by setting the `port` field.
///
/// See the [Dockerhub](https://hub.docker.com/r/hashicorp/counting-service)
/// page for more information on the arguments and environment variables that
/// can be used to configure the server.
#[derive(Clone, Default, Builder)]
#[builder(default)]
pub struct CountingServerConfig {
    #[builder(default = "Vec::new()")]
    pub args: Vec<String>,
    #[builder(default = "HashMap::new()")]
    pub env: HashMap<String, String>,
    #[builder(default = "crate::server::new_handle(IMAGE)")]
    pub handle: String,
    #[builder(default = "9500")]
    pub port: u32,
    #[builder(default = "15")]
    pub timeout: u16,
    #[builder(default = "rand_string(16)")]
    pub token: String,
    #[builder(default = "String::from(\"0.0.2\")")]
    pub version: String,
}

impl CountingServerConfig {
    pub fn builder() -> CountingServerConfigBuilder {
        CountingServerConfigBuilder::default()
    }
}

impl Config for CountingServerConfig {
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
        }
        .into()
    }

    fn handle(&self) -> &str {
        self.handle.as_str()
    }
}

/// A running instance of a Counting server.
pub struct CountingServer {
    pub external_port: u32,
    pub internal_port: u32,
    pub ip: String,
}

impl CountingServer {
    fn format_address(&self, host: &str, port: u32) -> String {
        format!("{}:{}", host, port)
    }

    fn format_url(&self, host: &str, port: u32) -> String {
        format!("http://{}", self.format_address(host, port))
    }

    /// The external address in the form of localhost::{port}
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

impl Server for CountingServer {
    type Config = CountingServerConfig;

    fn new(config: &Self::Config, container: &dockertest::RunningContainer) -> Self {
        CountingServer {
            external_port: config.port,
            internal_port: PORT,
            ip: container.ip().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::{CountingServer, CountingServerConfig};
    use crate::Test;

    const PORT: u32 = 9001;

    #[test]
    fn test_counting() {
        let config = CountingServerConfig::builder().port(PORT).build().unwrap();
        let mut test = Test::new();
        test.register(config);

        test.run(|instance| async move {
            let server: CountingServer = instance.server();

            let client = reqwest::Client::new();
            let resp = client.get(server.external_url()).send().await;
            assert!(resp.is_ok());
            assert_eq!(resp.unwrap().status(), 200);
        });
    }
}
