use crate::{Config, ContainerConfig, Server};
use derive_builder::Builder;
use dockertest::{waitfor, PullPolicy, Source};
use std::collections::HashMap;
use tempfile::{TempPath, NamedTempFile};
use std::io::Write;

const IMAGE: &str = "nginx";
const PORT: u32 = 8888;
const LOG_MSG: &str = "start worker process";
const SOURCE: Source = Source::DockerHub(PullPolicy::IfNotPresent);

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[derive(Clone, Default, Builder)]
#[builder(default)]
pub struct WebserverContent {
    #[builder(default = "String::from(\"text/html\")")]
    pub content_type: String,
    #[builder(default = "Vec::new()")]
    pub content: Vec<u8>,
    #[builder(default = "String::new()")]
    pub serve_path: String,
}

impl WebserverContent {

    pub fn builder() -> WebserverContentBuilder {
        WebserverContentBuilder::default()
    }

}

pub struct ManagedContent {
    pub config: TempPath,
    pub content: TempPath,
}

#[derive(Debug)]
pub enum ContentError {
    MalformedPath,
    IO(std::io::Error),
}

impl std::convert::From<std::io::Error> for ContentError {
    fn from(inner: std::io::Error) -> ContentError {
        ContentError::IO(inner)
    }
}

#[derive(Clone, Default, Builder)]
#[builder(default)]
pub struct NginxServerConfig {
    #[builder(default = "Vec::new()")]
    pub args: Vec<String>,
    #[builder(default = "HashMap::new()")]
    pub env: HashMap<String, String>,
    #[builder(default = "crate::server::new_handle(IMAGE)")]
    pub handle: String,
    #[builder(default = "8888")]
    pub port: u32,
    #[builder(default = "10")]
    pub timeout: u16,
    #[builder(default = "String::from(\"latest\")")]
    pub version: String,
    #[builder(default = "HashMap::new()")]
    pub bind_mounts: HashMap<String, String>,
}

impl NginxServerConfig {
    pub fn builder() -> NginxServerConfigBuilder {
        NginxServerConfigBuilder::default()
    }

    pub fn add_mount(&mut self, local_path: &str, remote_path: &str) {
        self.bind_mounts.insert(remote_path.to_string(), local_path.to_string());
    }

    pub fn add_config_file(&mut self, name: &str, local_path: &str) {
        let name = format!("/etc/nginx/conf.d/{}", &name);
        self.add_mount(local_path, &name);
    }

    pub fn add_config(&mut self, verbatim_config: &str) -> Result<NamedTempFile, ContentError> {
        let mut tempfile = tempfile::Builder::new()
            .prefix("nginx")
            .suffix(".conf")
            .rand_bytes(10)
            .tempfile()?;

        let (local_path, tempfile_base) = Self::tempfile_name_parts(&tempfile)?;

        tempfile.write_all(verbatim_config.as_bytes())?;
        self.add_config_file(&tempfile_base, &local_path);
        Ok(tempfile)
    }

    pub fn add_web_content(&mut self, content: WebserverContent) -> Result<ManagedContent, ContentError> {
        
        //always shadow the upstream image default site when adding custom content
        self.shadow_upstream_default_site();

        let mut tempfile = tempfile::Builder::new()
            .prefix("content")
            .rand_bytes(10)
            .tempfile()?;

        let (local_path, tempfile_base) = Self::tempfile_name_parts(&tempfile)?;
        let remote_path = format!("/usr/share/nginx/html/{}", &tempfile_base);
    
        tempfile.write_all(&content.content)?;
        self.add_mount(&local_path, &remote_path);
        
        let temp_path = tempfile.into_temp_path();
        
        {
            // this has be done for content to be readable by the webserver
            #[cfg(unix)]
            std::fs::set_permissions(&temp_path, std::fs::Permissions::from_mode(0o644))?;
        }

        let config = self.add_config(&format!(r#"
            server {{
                listen {port} default_server;
                location ={location} {{
                    default_type {content_type};
                    alias {alias};
                }}
            }}
        "#,
            port=PORT,
            location=&content.serve_path,
            content_type=&content.content_type,
            alias=&remote_path
        ))?.into_temp_path();

        Ok(ManagedContent{
            config: config,
            content: temp_path,
        })
    }

    // idempotent since bind mounts are key'ed by their target path
    pub fn shadow_upstream_default_site(&mut self) {
        self.add_config_file("default.conf", "/dev/null");
    }

    fn tempfile_name_parts(file: &NamedTempFile) -> Result<(String, String), ContentError> {
        let full_path = file
            .path()
            .to_str()
            .ok_or(ContentError::MalformedPath)?
            .to_string();
        let base_name = file
            .path()
            .file_name()
            .ok_or(ContentError::MalformedPath)?
            .to_str()
            .ok_or(ContentError::MalformedPath)?
            .to_string();
        
        Ok((full_path, base_name))
    }
}

impl Config for NginxServerConfig {
    fn into_composition(self) -> dockertest::Composition {
        let ports = vec![(PORT, self.port)];

        let timeout = self.timeout;
        let wait = Box::new(waitfor::MessageWait {
            message: LOG_MSG.into(),
            source: waitfor::MessageSource::Stderr,
            timeout,
        });

        ContainerConfig {
            args: Vec::new(),
            env: HashMap::new(),
            handle: self.handle,
            name: IMAGE.into(),
            source: SOURCE,
            version: self.version,
            ports: Some(ports),
            wait: Some(wait),
            bind_mounts: self.bind_mounts,
        }
        .into()
    }

    fn handle(&self) -> &str {
        self.handle.as_str()
    }
}

pub struct NginxServer {
    pub external_port: u32,
    pub internal_port: u32,
    pub ip: String,
}

impl NginxServer {
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

impl Server for NginxServer {
    type Config = NginxServerConfig;

    fn new(config: &Self::Config, container: &dockertest::RunningContainer) -> Self {
        NginxServer {
            external_port: config.port,
            internal_port: PORT,
            ip: container.ip().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{NginxServer, NginxServerConfig, WebserverContent};
    use crate::Test;
    use test_env_log::test;

    const PORT: u32 = 8888;

    #[test]
    fn test_basic() {
        let mut config = NginxServerConfig::builder()
            .port(PORT)
            .build()
            .unwrap();

        let payload = r#"{"hello": "world!"}"#.to_string();

        let _hello_world = config.add_web_content(
            WebserverContent::builder()
                .content(payload.as_bytes().to_vec())
                .content_type("application/json".to_string())
                .serve_path("/hello".to_string())
                .build()
                .unwrap()
        ).unwrap();
    
        let mut test = Test::new();
        test.register(config);

        test.run(|instance| async move {
            let server: NginxServer = instance.server();

            let client = reqwest::Client::new();
            let resp = client
                .get(format!(
                    "{}/hello",
                    server.external_url()
                ))
                .send()
                .await.unwrap()
                .text()
                .await.unwrap();
        
            assert_eq!(&resp, &payload);
        });
    }
}
