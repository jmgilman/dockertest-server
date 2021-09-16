mod nginx;

use dockertest_server::Test;
use nginx::{NginxServer, NginxServerConfig};

#[test]
fn test_vault() {
    let config = NginxServerConfig::builder().build().unwrap();
    let mut test = Test::new();
    test.register(config);

    test.run(|instance| async move {
        let server: NginxServer = instance.server();

        let client = reqwest::Client::new();
        let resp = client.get(server.local_address).send().await;
        assert!(resp.is_ok());
        assert_eq!(resp.unwrap().status(), 200);
    });
}
