/// Contains types for creating tests
use dockertest::{Composition, DockerOperations, DockerTest};
use futures::Future;
use type_map::concurrent::TypeMap;

use crate::server::{Config, Server};

/// A single test which brings up one or more [Servers][Server].
///
/// A [Test][crate::test::Test] receives [Configs][Config] via `register` which
/// determines the servers that will be brought up when the test is executed.
/// The `run` method is used to bring up the [Servers][Server] and then run the
/// given test body.
///
/// The body of a test is determined by an async closure passed to the `run`
/// method. The closure will receive a [TestInstance] which can be used to
/// access the [Servers][Server] that were created when the test was setup.
pub struct Test {
    pub configs: TypeMap,
    pub compositions: Vec<Composition>,
}

impl Test {
    /// Creates an empty [Test].
    pub fn new() -> Self {
        Test {
            configs: TypeMap::new(),
            compositions: Vec::new(),
        }
    }

    /// Registers a [Config] with this test.
    ///
    /// A [Test] can be configured with any number of [Configs][Config] for
    /// determining which [Servers][Server] are brought up in a test. Each
    /// [Config] passed will have it's respective [Server] created before the
    /// test body is ran.
    pub fn register(&mut self, config: impl Config + 'static) {
        self.configs.insert(config.clone());
        self.compositions.push(config.into_composition());
    }

    /// Brings up the [Servers][Server] registered with this test and then
    /// runs the given test body.
    ///
    /// The test body receives a [TestInstance] which can be used for accessing
    /// any configured [Servers][Server] through the `server` method.
    /// The test body is guaranteed to not execute until all configured
    /// [Servers][Server] are verified to be running and available. The scope of
    /// the test body determines the life of the [Servers][Server]: they are
    /// created before the closure is run and destroyed after the closure exits.
    pub fn run<T, F>(self, fun: T)
    where
        T: FnOnce(TestInstance) -> F + Send + 'static,
        F: Future<Output = ()> + Send + 'static,
    {
        let mut test = DockerTest::new();
        for comp in self.compositions {
            test.add_composition(comp)
        }

        let configs = self.configs;
        test.run(|ops| async move {
            let instance = TestInstance::new(configs, ops);
            (fun)(instance).await;
        });
    }
}

impl Default for Test {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents a running instance of a [Test].
///
/// Internally, this type keeps track of all [Configs][Config] that were
/// registered with a [Test] using a [TypeMap]. A [Server] can be fetched by
/// calling the `server` method with the desired type that implements [Server].
/// See the associated method documentation for more details.
pub struct TestInstance {
    pub configs: TypeMap,
    pub op: DockerOperations,
}

impl TestInstance {
    /// Returns a new [TestInstance].
    pub fn new(configs: TypeMap, op: DockerOperations) -> Self {
        TestInstance { configs, op }
    }

    /// Returns an instance of the requested type that implements [Server].
    ///
    /// Internally a [TestInstance] has a [TypeMap] which contains all of the
    /// [Configs][Config] that were registered with a given [Test]. Since each
    /// [Config] has a one-to-one relationship with a [Server], an instance of a
    /// [Server] can be created by simply having a copy of it's associated
    /// [Config]. This method performs that mapping for you, looking for a
    /// [Config] of the type specified by [Server::Config] in the type map and
    /// then generating and returning a new instance of the [Server] using the
    /// [Config] and runtime data generated when the underlying container was
    /// brought up.
    ///
    /// Note that it then follows this method will fail if the passed [Server]
    /// type did not have it's associated [Config] registered when the [Test]
    /// was created.
    pub fn server<S: Server>(&self) -> S {
        let config = self.configs.get::<S::Config>().unwrap();
        let container = self.op.handle(config.handle());
        S::new(config, container)
    }
}
