use std::collections::HashMap;

use dockertest::{waitfor, Composition, Image, RunningContainer, Source};

/// A configuration capable of configuring a [Server].
///
/// Types that implement this trait are intended to have a one-to-one
/// relationship with a [Server] as specified by [Server::Config]. The
/// implementation is left intentionally sparse in order to maximize the process
/// of creating a container [Composition]. The [generate_composition] helper
/// function is provided for creating a [Composition] from the usual required
/// configuration options.
///
/// See also [Test][crate::test::Test].
pub trait Config: Send + Sync {
    fn composition(&self) -> Composition;
    fn handle(&self) -> &str;
}

/// A running instance of a specific container generated by a [Config].
///
/// Types that implement this trait are intended to have a one-to-one
/// relationship with a [Config] as specified by [Server::Config]. When a
/// [Test][crate::test::Test] is created it is passed one or more
/// [Configs][Config] which determine what containers are brought up during the
/// test. A [Server] represents a running container within the context of a
/// single test. This is reflected by the fact that a [Server] is created using
/// a [Config] as well as the runtime data provided by a [RunningContainer].
///
/// Types implementing this trait should provide as much utility to the end-user
/// as possible for interacting with the running container. For example, if the
/// container is a web server, this trait should provide functionality for
/// obtaining it's URL.
pub trait Server {
    type Config: Config + 'static;

    fn new(config: &Self::Config, container: &RunningContainer) -> Self;
}

/// A helper function for generating [Compositions][Composition].
///
/// A [Composition] usually consists of a few common configuration properties.
/// This helper function is intended to provide a common interface to those
/// properties to assist upstream [Configs][Config] with creating them.
pub fn generate_composition(
    args: Vec<String>,
    env: HashMap<String, String>,
    handle: &str,
    name: &str,
    source: Source,
    timeout: u16,
    version: &str,
    ports: Option<Vec<(u32, u32)>>,
    wait_msg: Option<&str>,
) -> Composition {
    let image = Image::with_repository(name).source(source).tag(version);
    let mut comp = Composition::with_image(image);

    let wait = if let Some(msg) = wait_msg {
        Some(Box::new(waitfor::MessageWait {
            message: String::from(msg),
            source: waitfor::MessageSource::Stdout,
            timeout: timeout,
        }))
    } else {
        None
    };

    if let Some(p) = ports {
        for pair in p {
            comp.port_map(pair.0, pair.1);
        }
    };

    match wait {
        Some(w) => comp
            .with_cmd(args)
            .with_env(env)
            .with_wait_for(w)
            .with_container_name(handle),
        None => comp
            .with_cmd(args)
            .with_env(env)
            .with_container_name(handle),
    }
}

/// A helper function for generating random handles.
///
/// The returned handle is a combination of the given name and a random 10
/// character string.
pub fn new_handle(name: &str) -> String {
    format!("{}{}", name, crate::common::rand_string(10))
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_new_handle() {
        let result = super::new_handle("test");
        assert_eq!(result.len(), 14);
    }
}