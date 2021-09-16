pub mod common;
pub mod server;
pub mod test;
pub mod vault;

pub use server::{generate_composition, new_handle, Config, Server};
pub use test::{Test, TestInstance};

#[cfg(test)]
mod tests {

    use super::test::Test;
    use super::vault::{VaultServer, VaultServerConfig};

    #[test]
    fn test_vault() {
        let config = VaultServerConfig::builder()
            .port(8300)
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
