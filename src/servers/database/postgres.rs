use crate::common::rand_string;
use crate::{Config, ContainerConfig, Server};
use derive_builder::Builder;
use dockertest::{waitfor, PullPolicy, Source};
use std::collections::HashMap;

const IMAGE: &str = "postgres";
const PORT: u32 = 5432;
const LOG_MSG: &str = "database system is ready to accept connections";
const SOURCE: Source = Source::DockerHub(PullPolicy::IfNotPresent);
const USER: &str = "postgres";

/// Configuration for creating a PostgreSQL server.
///
/// By default the PostgreSQL server listens on port 5432 for requests. This
/// is exposed on the container by default, but the exposed port can be
/// controlled by setting the `port` field.
///
/// See the [DockerHub](https://hub.docker.com/_/postgres) repo for more
/// information on the arguments and environment variables that can be used to
/// configure the server.
#[derive(Clone, Default, Builder)]
#[builder(default)]
pub struct PostgresServerConfig {
    #[builder(default = "Vec::new()")]
    pub args: Vec<String>,
    #[builder(default = "HashMap::new()")]
    pub env: HashMap<String, String>,
    #[builder(default = "crate::server::new_handle(IMAGE)")]
    pub handle: String,
    #[builder(default = "rand_string(16)")]
    pub password: String,
    #[builder(default = "8200")]
    pub port: u32,
    #[builder(default = "15")]
    pub timeout: u16,
    #[builder(default = "String::from(\"latest\")")]
    pub version: String,
}

impl PostgresServerConfig {
    pub fn builder() -> PostgresServerConfigBuilder {
        PostgresServerConfigBuilder::default()
    }
}

impl Config for PostgresServerConfig {
    fn into_composition(self) -> dockertest::Composition {
        let ports = vec![(PORT, self.port)];

        let mut env = self.env.clone();
        env.insert(String::from("POSTGRES_PASSWORD"), self.password.clone());

        let mut args = self.args.clone();
        args.push("-c".into());
        args.push("listen_addresses=*".into());

        let timeout = self.timeout;
        let wait = Box::new(waitfor::MessageWait {
            message: LOG_MSG.into(),
            source: waitfor::MessageSource::Stderr,
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

/// A running instance of a PostgreSQL server.
///
/// The server address which is accessible from the local host can be found in
/// `local_address`. Other running containers which need access to this server
/// should use the `address` field instead.
pub struct PostgresServer {
    pub external_port: u32,
    pub internal_port: u32,
    pub ip: String,
    pub password: String,
    pub username: String,
}

impl PostgresServer {
    fn format_address(&self, host: &str, port: u32) -> String {
        format!("{}:{}", host, port)
    }

    fn format_auth_url(&self, host: &str, port: u32) -> String {
        format!(
            "postgresql://{}:{}@{}",
            self.username,
            self.password,
            self.format_address(host, port)
        )
    }

    fn format_url(&self, host: &str, port: u32) -> String {
        format!("postgresql://{}", self.format_address(host, port))
    }

    /// The external address in the form of localhost:{port}
    pub fn external_address(&self) -> String {
        self.format_address("localhost", self.external_port)
    }

    /// The external libpq URL with the username/password embedded in the URL
    pub fn external_auth_url(&self) -> String {
        self.format_auth_url("localhost", self.external_port)
    }

    /// The external libpq URL
    pub fn external_url(&self) -> String {
        self.format_url("localhost", self.external_port)
    }

    /// The container internal address in the form of {ip}:{port}
    pub fn internal_address(&self) -> String {
        self.format_address(self.ip.as_str(), self.internal_port)
    }

    /// The internal libpq URL with the username/password embedded in the URL
    pub fn internal_auth_url(&self) -> String {
        self.format_auth_url(self.ip.as_str(), self.internal_port)
    }

    /// The internal libpq URL
    pub fn internal_url(&self) -> String {
        self.format_url(self.ip.as_str(), self.internal_port)
    }
}

impl Server for PostgresServer {
    type Config = PostgresServerConfig;

    fn new(config: &Self::Config, container: &dockertest::RunningContainer) -> Self {
        PostgresServer {
            external_port: config.port,
            internal_port: PORT,
            ip: container.ip().to_string(),
            password: config.password.clone(),
            username: USER.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PostgresServer, PostgresServerConfig};
    use crate::Test;
    use test_env_log::test;
    use tokio_postgres::NoTls;

    const PORT: u32 = 6432;

    #[test]
    fn test_postgres() {
        let config = PostgresServerConfig::builder().port(PORT).build().unwrap();
        let mut test = Test::new();
        test.register(config);

        test.run(|instance| async move {
            let server: PostgresServer = instance.server();
            let res = tokio_postgres::connect(server.external_auth_url().as_str(), NoTls).await;
            assert!(res.is_ok())
        });
    }
}
