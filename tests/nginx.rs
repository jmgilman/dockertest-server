use derive_builder::Builder;
use dockertest::{PullPolicy, Source};
use dockertest_server::{Config, Server};
use std::collections::HashMap;

const IMAGE: &str = "nginx";
const PORT: u32 = 80;
const LOG_MSG: &str = "/docker-entrypoint.sh: Configuration complete; ready for start up";
const SOURCE: Source = Source::DockerHub(PullPolicy::IfNotPresent);

#[derive(Default, Builder)]
#[builder(default)]
pub struct NginxServerConfig {
    #[builder(default = "Vec::new()")]
    pub args: Vec<String>,
    #[builder(default = "HashMap::new()")]
    pub env: HashMap<String, String>,
    #[builder(default = "dockertest_server::new_handle(IMAGE)")]
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

        dockertest_server::server::generate_composition(
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
