use crate::common::rand_string;
use crate::{Config, ContainerConfig, Server};
use derive_builder::Builder;
use dockertest::{waitfor, Source};
use std::collections::HashMap;

const IMAGE: &str = "redis";
const PORT: u32 = 6379;
const LOG_MSG: &str = "Ready to accept connections tcp";
const SOURCE: Source = Source::DockerHub;

/// Configuration for creating a Redis server.
///
/// By default the Redis server listens on port 6379 for requests. This
/// is exposed on the container by default, but the exposed port can be
/// controlled by setting the `port` field.
///
/// See the [DockerHub](https://hub.docker.com/_/redis) repo for more
/// information on the arguments and environment variables that can be used to
/// configure the server.
#[derive(Clone, Default, Builder)]
#[builder(default)]
pub struct RedisServerConfig {
    #[builder(default = "Vec::new()")]
    pub args: Vec<String>,
    #[builder(default = "HashMap::new()")]
    pub env: HashMap<String, String>,
    #[builder(default = "crate::server::new_handle(IMAGE)")]
    pub handle: String,
    #[builder(default = "6379")]
    pub port: u32,
    #[builder(default = "15")]
    pub timeout: u16,
    #[builder(default = "String::from(\"latest\")")]
    pub version: String,
}

impl RedisServerConfig {
    pub fn builder() -> RedisServerConfigBuilder {
        RedisServerConfigBuilder::default()
    }
}

impl Config for RedisServerConfig {
    fn into_composition(self) -> dockertest::Composition {
        let ports = vec![(PORT, self.port)];

        let env = self.env.clone();
        let args = self.args.clone();

        let timeout = self.timeout;
        let wait = Box::new(waitfor::MessageWait {
            message: LOG_MSG.into(),
            source: waitfor::MessageSource::Stdout,
            timeout,
        });

        ContainerConfig {
            args,
            env,
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

/// A running instance of a Redis server.
///
/// The server address which is accessible from the local host can be found in
/// `local_address`. Other running containers which need access to this server
/// should use the `address` field instead.
pub struct RedisServer {
    pub external_port: u32,
    pub internal_port: u32,
    pub ip: String,
}

impl RedisServer {
    fn format_address(&self, host: &str, port: u32) -> String {
        format!("{}:{}", host, port)
    }

    fn format_url(&self, host: &str, port: u32) -> String {
        format!("redis://{}", self.format_address(host, port))
    }

    /// The external address in the form of localhost:{port}
    pub fn external_address(&self) -> String {
        self.format_address("localhost", self.external_port)
    }

    /// The external libpq URL
    pub fn external_url(&self) -> String {
        self.format_url("localhost", self.external_port)
    }

    /// The container internal address in the form of {ip}:{port}
    pub fn internal_address(&self) -> String {
        self.format_address(self.ip.as_str(), self.internal_port)
    }

    /// The internal libpq URL
    pub fn internal_url(&self) -> String {
        self.format_url(self.ip.as_str(), self.internal_port)
    }
}

impl Server for RedisServer {
    type Config = RedisServerConfig;

    fn new(config: &Self::Config, container: &dockertest::RunningContainer) -> Self {
        RedisServer {
            external_port: config.port,
            internal_port: PORT,
            ip: container.ip().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{RedisServer, RedisServerConfig};
    use crate::Test;
    use test_log::test;

    const PORT: u32 = 9898;

    #[test]
    fn test_redis() {
        let config = RedisServerConfig::builder().port(PORT).build().unwrap();
        let mut test = Test::new();
        test.register(config);

        test.run(|instance| async move {
            let server: RedisServer = instance.server();
            let client = redis::Client::open(server.external_url().as_str()).unwrap();
            let mut con = client.get_connection().unwrap();
            let res: String = redis::cmd("PING").query(&mut con).unwrap();
            assert_eq!(res, "PONG");
        });
    }
}
