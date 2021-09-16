use crate::{Config, Server};
use derive_builder::Builder;
use dockertest::{PullPolicy, Source};
use std::collections::HashMap;

const IMAGE: &str = "nginx";
const PORT: u32 = 80;
const LOG_MSG: &str = "/docker-entrypoint.sh: Configuration complete; ready for start up";
const SOURCE: Source = Source::DockerHub(PullPolicy::IfNotPresent);

/// Configuration for creating an Nginx web server
///
/// If no port is specified, defaults to exposing the server on port 8080.
///
/// See the [Dockerhub](https://hub.docker.com/_/nginx) page for more
/// information on the arguments and environment variables that can be used to
/// configure the server.
#[derive(Default, Builder)]
#[builder(default, setter(into))]
pub struct NginxServerConfig {
    #[builder(default = "Vec::new()")]
    pub args: Vec<String>,
    #[builder(default = "HashMap::new()")]
    pub env: HashMap<String, String>,
    #[builder(default = "crate::new_handle(IMAGE)")]
    pub handle: String,
    #[builder(default = "8080")]
    pub port: u32,
    #[builder(default = "15")]
    pub timeout: u16,
    #[builder(default = "String::from(\"latest\")")]
    pub version: String,
}

impl NginxServerConfig {
    pub fn builder() -> NginxServerConfigBuilder {
        NginxServerConfigBuilder::default()
    }
}

impl Config for NginxServerConfig {
    fn composition(&self) -> dockertest::Composition {
        let ports = vec![(PORT, self.port)];

        crate::server::generate_composition(
            self.args.clone(),
            self.env.clone(),
            self.handle.as_str(),
            IMAGE,
            SOURCE,
            self.timeout,
            self.version.as_str(),
            Some(ports),
            Some(LOG_MSG),
        )
    }

    fn handle(&self) -> &str {
        self.handle.as_str()
    }
}

/// A running instane of a Nginx server.
///
/// The server URL which is accessible from the local host can be found in
/// `local_address`. Other running containers which need access to this server
/// should use the `address` field instead.
pub struct NginxServer {
    pub address: String,
    pub local_address: String,
    pub port: u32,
}

impl Server for NginxServer {
    type Config = NginxServerConfig;

    fn new(config: &Self::Config, container: &dockertest::RunningContainer) -> Self {
        NginxServer {
            address: format!("http://{}:{}", container.ip(), config.port),
            local_address: format!("http://localhost:{}", config.port),
            port: config.port,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{NginxServer, NginxServerConfig};
    use crate::Test;

    #[test]
    fn test_nginx() {
        let config = NginxServerConfig::builder()
            .version("1.21.3-alpine")
            .port(8082 as u32)
            .build()
            .unwrap();
        let mut test = Test::new();
        test.register(config);

        test.run(|instance| async move {
            let server: NginxServer = instance.server();

            let resp = reqwest::get(server.local_address).await;
            assert!(resp.is_ok());
            assert_eq!(resp.unwrap().status(), 200);
        });
    }
}
