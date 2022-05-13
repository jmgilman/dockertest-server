# dockertest-server

<p align="center">
    <a href="https://crates.io/crates/dockertest-server">
        <img src="https://img.shields.io/crates/v/dockertest-server">
    </a>
    <a href="https://docs.rs/dockertest-server">
        <img src="https://img.shields.io/docsrs/dockertest-server" />
    </a>
    <a href="https://github.com/jmgilman/dockertest-server/actions/workflows/ci.yml">
        <img src="https://github.com/jmgilman/dockertest-server/actions/workflows/ci.yml/badge.svg"/>
    </a>
</p>

> A test framework built around [dockertest][1] for testing against server containers.

This crate provides a small abstraction layer around the
[dockertest][1] crate for easily running a test against multiple servers running
in containers. It provides traits for defining a server and it's associated
configuration and then registering a variable number of servers to bring up for
a test.

The primary use-case of this crate is to provide an easy-to-use testing
framework for crates that wish to build integration tests against services which
are able to run in a container.

## Installation

Add `dockertest-server` as a dependency to your cargo.toml:

```toml
[dev-dependencies]
dockertest-server = "0.1.7"
```

## Usage

The below example brings up a mock OAuth server and then tests it's responding
to HTTP requests:

```rust
// Note: This requires the `web` feature
use dockertest_server::servers::auth::{OIDCServer, OIDCServerConfig};
use dockertest_server::Test;

let config = OIDCServerConfig::builder().port(8090).build().unwrap();
let mut test = Test::new();
test.register(config);

test.run(|instance| async move {
    let server: OIDCServer = instance.server();

    let client = reqwest::Client::new();
    let resp = client
        .get(format!(
            "{}/default/.well-known/openid-configuration",
            server.external_url()
        ))
        .send()
        .await;
    assert!(resp.is_ok());
    assert_eq!(resp.unwrap().status(), 200);
});
```

This crate ships with support for various servers already included. See the
`servers` module for the ones included. Note that most require a feature flag to
be enabled to avoid bundling unnecessary implementations.

Please feel free to submit a PR with your own implementations to be added to the
main crate.

## Testing

Run tests with `cargo test`.

## Contributing

Check out the [issues][2] for items needing attention or submit your own and
then:

1. Fork the repo (<https://github.com/jmgilman/dockertest-server/fork>)
2. Create your feature branch (git checkout -b feature/fooBar)
3. Commit your changes (git commit -am 'Add some fooBar')
4. Push to the branch (git push origin feature/fooBar)
5. Create a new Pull Request

[1]: https://crates.io/crates/dockertest
[2]: https://github.com/jmgilman/dockertest-server/issues
