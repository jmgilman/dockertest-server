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

## Installation

Add `dockertest-server` as a depdendency to your cargo.toml:
```
[dev-dependencies]
dockertest-server = "0.1.0"
```

## Testing

Run tests with `cargo test`.

Check out the [issues][2] for items neeeding attention or submit your own and 
then:

1. Fork the repo (https://github.com/jmgilman/dockertest-server/fork)
2. Create your feature branch (git checkout -b feature/fooBar)
3. Commit your changes (git commit -am 'Add some fooBar')
4. Push to the branch (git push origin feature/fooBar)
5. Create a new Pull Request

[1]: https://crates.io/crates/dockertest
[2]: https://github.com/jmgilman/dockertest-server/issues
