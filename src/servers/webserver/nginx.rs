use crate::{Config, ContainerConfig, Server};
use derive_builder::Builder;
use dockertest::{waitfor, PullPolicy, Source};
use std::collections::HashMap;
use std::io::Write;
use tempfile::{NamedTempFile, TempPath};

const IMAGE: &str = "nginx";
const PORT: u32 = 8888;
const LOG_MSG: &str = "start worker process";
const SOURCE: Source = Source::DockerHub(PullPolicy::IfNotPresent);

use std::fs::Permissions;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[derive(Clone, Default, Builder)]
#[builder(default)]
pub struct WebserverContent {
    #[builder(default = "String::new()", setter(into))]
    pub name: String,
    #[builder(default = "String::from(\"text/html\")", setter(into))]
    pub content_type: String,
    #[builder(default = "Vec::new()")]
    pub content: Vec<u8>,
    #[builder(default = "String::new()", setter(into))]
    pub serve_path: String,
}

impl WebserverContent {
    pub fn builder() -> WebserverContentBuilder {
        WebserverContentBuilder::default()
    }
}

pub type ManagedContent = Vec<TempPath>;

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

#[derive(Clone)]
pub struct TlsConfig {
    cert_path: String,
    key_path: String,
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
    #[builder(default)]
    pub tls: Option<TlsConfig>,
}

impl NginxServerConfig {
    pub fn builder() -> NginxServerConfigBuilder {
        NginxServerConfigBuilder::default()
    }

    pub fn tls_from_ca_bytes(
        &mut self,
        cert: &[u8],
        key: &[u8],
    ) -> Result<ManagedContent, ContentError> {
        let mut content = Vec::new();

        content.push(self.tempfile_mount("ca", ".crt", cert, "/srv/ca.crt")?);
        content.push(self.tempfile_mount("ca", ".key", key, "/srv/ca.key")?);
        content.push(self.tempfile_mount(
            "csr",
            ".cnf",
            include_bytes!("./openssl-csr.cnf"),
            "/srv/openssl-csr.cnf",
        )?);
        content.push(self.tempfile_mount(
            "signing",
            ".cnf",
            include_bytes!("./openssl-signing.cnf"),
            "/srv/openssl-signing.cnf",
        )?);
        content.push(self.tempfile_mount_executable(
            "certgenerate",
            ".sh",
            include_bytes!("./generate-cert.sh"),
            "/docker-entrypoint.d/40-generate-cert.sh",
        )?);

        self.tls = Some(TlsConfig {
            cert_path: "/srv/webserver.crt".to_string(),
            key_path: "/srv/webserver.key".to_string(),
        });

        Ok(content)
    }

    pub fn add_mount(&mut self, local_path: &str, remote_path: &str) {
        self.bind_mounts
            .insert(remote_path.to_string(), local_path.to_string());
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

    pub fn add_web_content(
        &mut self,
        content: WebserverContent,
    ) -> Result<ManagedContent, ContentError> {
        //always shadow the upstream image default site when adding custom content
        self.shadow_upstream_default_site();

        let remote_path = format!("/usr/share/nginx/html/{}", &content.name);
        let temp_path =
            self.tempfile_mount(&content.name, "content", &content.content, &remote_path)?;

        let optional_tls_config = match &self.tls {
            Some(c) => format!(
                r#"ssl default_server;
                    server_name localhost;
                    ssl_certificate     {cert};
                    ssl_certificate_key {key};
                "#,
                cert = &c.cert_path,
                key = &c.key_path
            ),
            None => "default_server;\n".to_string(),
        };

        let config = self
            .add_config(&format!(
                r#"
            server {{
                listen {port} {tls_config}
                location ={location} {{
                    default_type {content_type};
                    alias {alias};
                }}

                # hack to allow all http methods on static resources
                error_page  405     =200 $uri;
            }}
        "#,
                port = PORT,
                location = &content.serve_path,
                content_type = &content.content_type,
                alias = &remote_path,
                tls_config = &optional_tls_config
            ))?
            .into_temp_path();

        Ok(vec![config, temp_path])
    }

    // idempotent since bind mounts are key'ed by their target path
    pub fn shadow_upstream_default_site(&mut self) {
        self.add_config_file("default.conf", "/dev/null");
    }

    fn tempfile_mount(
        &mut self,
        prefix: &str,
        suffix: &str,
        content: &[u8],
        target_path: &str,
    ) -> Result<TempPath, ContentError> {
        self.tempfile_mount_with_perms(
            prefix,
            suffix,
            content,
            target_path,
            std::fs::Permissions::from_mode(0o644),
        )
    }

    fn tempfile_mount_executable(
        &mut self,
        prefix: &str,
        suffix: &str,
        content: &[u8],
        target_path: &str,
    ) -> Result<TempPath, ContentError> {
        self.tempfile_mount_with_perms(
            prefix,
            suffix,
            content,
            target_path,
            std::fs::Permissions::from_mode(0o755),
        )
    }

    fn tempfile_mount_with_perms(
        &mut self,
        prefix: &str,
        suffix: &str,
        content: &[u8],
        target_path: &str,
        permissions: Permissions,
    ) -> Result<TempPath, ContentError> {
        let mut file = tempfile::Builder::new()
            .prefix(prefix)
            .suffix(suffix)
            .rand_bytes(10)
            .tempfile()?;

        file.write_all(content)?;

        #[cfg(unix)]
        std::fs::set_permissions(&file, permissions)?;
        self.add_mount(
            file.path().to_str().ok_or(ContentError::MalformedPath)?,
            target_path,
        );
        Ok(file.into_temp_path())
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
    pub with_tls: bool,
}

impl NginxServer {
    fn format_address(&self, host: &str, port: u32) -> String {
        format!("{}:{}", host, port)
    }

    fn format_url(&self, host: &str, port: u32) -> String {
        let scheme = if self.with_tls { "https" } else { "http" };
        format!(
            "{scheme}://{address}",
            scheme = scheme,
            address = self.format_address(host, port)
        )
    }

    /// The external address in the form of localhost:{port}
    pub fn external_address(&self) -> String {
        self.format_address("localhost", self.external_port)
    }

    pub fn external_url(&self) -> String {
        self.format_url("localhost", self.external_port)
    }

    /// The container internal address in the form of {ip}:{port}
    pub fn internal_address(&self) -> String {
        self.format_address(self.ip.as_str(), self.internal_port)
    }

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
            with_tls: config.tls.is_some(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{NginxServer, NginxServerConfig, WebserverContent};
    use crate::Test;
    use reqwest::Certificate;
    use test_log::test;

    #[test]
    fn test_basic() {
        let mut config = NginxServerConfig::builder().port(8888).build().unwrap();

        let payload = r#"{"hello": "world!"}"#.to_string();

        let _hello_world = config
            .add_web_content(
                WebserverContent::builder()
                    .name("hello")
                    .content(payload.as_bytes().to_vec())
                    .content_type("application/json")
                    .serve_path("/hello")
                    .build()
                    .unwrap(),
            )
            .unwrap();

        let mut test = Test::new();
        test.register(config);

        test.run(|instance| async move {
            let server: NginxServer = instance.server();

            let client = reqwest::Client::new();
            let resp = client
                .get(format!("{}/hello", server.external_url()))
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap();

            assert_eq!(&resp, &payload);
        });
    }

    #[test]
    fn test_tls() {
        let mut config = NginxServerConfig::builder().port(8443).build().unwrap();

        let _certs = config
            .tls_from_ca_bytes(include_bytes!("./ca.crt"), include_bytes!("./ca.key"))
            .unwrap();

        let payload = r#"{"hello": "world!"}"#.to_string();

        let _hello_world = config
            .add_web_content(
                WebserverContent::builder()
                    .name("hello")
                    .content(payload.as_bytes().to_vec())
                    .content_type("application/json")
                    .serve_path("/hello")
                    .build()
                    .unwrap(),
            )
            .unwrap();

        let mut test = Test::new();
        test.register(config);

        test.run(|instance| async move {
            let server: NginxServer = instance.server();

            let client = reqwest::Client::builder()
                .add_root_certificate(Certificate::from_pem(include_bytes!("./ca.crt")).unwrap())
                .build()
                .unwrap();

            let resp = client
                .post(format!("{}/hello", server.external_url()))
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap();

            assert_eq!(&resp, &payload);
        });
    }
}
