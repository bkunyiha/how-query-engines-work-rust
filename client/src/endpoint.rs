//! Per-server connection target: hostname + port, plus a helper that builds
//! the `http://host:port` URL tonic's `Channel::from_shared` needs.
//!
//! No Kotlin counterpart in `kquery/client/` — `Client.kt` takes `host` and
//! `port` as separate constructor arguments. The Rust port bundles them into
//! one `Endpoint` value so `Client`'s constructor takes a single descriptor
//! rather than two scalars, and so the `FlightExecutorClient`'s
//! `executor_id → Endpoint` map has a sensible value type. The Kotlin shape
//! is recoverable: an `Endpoint` is just `(host, port)` and Rust pattern
//! destructuring or two field reads round-trip the same information.

use distributed::ExecutorConfig;

/// One Flight server's connection details. Used both as the target for
/// `Client::new` and as the value type in `FlightExecutorClient`'s
/// `executor_id → Endpoint` map.
///
/// Derives `Clone`, `Debug`, `Eq`, `PartialEq`, `Hash` so callers can put
/// `Endpoint`s in `HashMap` keys / `HashSet`s if needed and so `Debug`
/// formatting works for tracing output.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Endpoint {
    pub host: String,
    pub port: i32,
}

impl Endpoint {
    /// Construct an endpoint from a host string and port number.
    pub fn new(host: impl Into<String>, port: i32) -> Self {
        Self {
            host: host.into(),
            port,
        }
    }

    /// Build the URL string `tonic::transport::Channel::from_shared` expects.
    /// Returns `http://host:port` — Flight does not use HTTPS in any of
    /// rquery's contexts (single-process tests + a teaching-port deployment
    /// would not configure TLS). HTTPS support would land alongside any
    /// production-deployment work.
    pub fn url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }
}

/// Convenience: build an `Endpoint` from a `distributed::ExecutorConfig`
/// (which already carries `host` + `port` for the scheduler's per-executor
/// dispatch). `FlightExecutorClient` uses this conversion to populate its
/// `executor_id → Endpoint` map from the `DistributedConfig.executors` list.
impl From<&ExecutorConfig> for Endpoint {
    fn from(config: &ExecutorConfig) -> Self {
        Self::new(&config.host, config.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_is_http_host_colon_port() {
        let ep = Endpoint::new("127.0.0.1", 50051);
        assert_eq!(ep.url(), "http://127.0.0.1:50051");
    }

    #[test]
    fn endpoint_round_trips_through_executor_config() {
        let cfg = ExecutorConfig::new("exec-1", "10.0.0.7", 50099);
        let ep = Endpoint::from(&cfg);
        assert_eq!(ep.host, "10.0.0.7");
        assert_eq!(ep.port, 50099);
    }

    #[test]
    fn endpoint_equality_holds_on_host_and_port() {
        // Two `Endpoint`s with the same host+port compare equal — used by
        // `FlightExecutorClient` to detect duplicate executor configurations.
        let a = Endpoint::new("localhost", 50051);
        let b = Endpoint::new("localhost", 50051);
        let c = Endpoint::new("localhost", 50052);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
