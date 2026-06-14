# proto/

Shared Protocol Buffers definitions. The `protobuf` crate compiles these into Rust types via `prost-build`. The generated types are then consumed by every crate that has `protobuf` as a dependency — currently `flight-server`, `client`, and `distributed`.

Generation is driven from the `protobuf` crate's `build.rs` via `prost-build`.

To regenerate:
```bash
cd ../protobuf && cargo build
```

The generated Rust code lands in `OUT_DIR` and is consumed via `include!(concat!(env!("OUT_DIR"), "/<package>.rs"))`.
