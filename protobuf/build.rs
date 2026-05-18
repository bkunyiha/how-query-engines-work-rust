//! Build-time codegen for the `.proto` files in `../../proto/`.
//! Generated Rust code lands in `OUT_DIR` and is `include!`'d from `src/lib.rs`.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // TODO: enable when `.proto` files are populated under ../../proto/
    //
    // let protos: &[&str] = &[
    //     "../../proto/plan.proto",
    //     "../../proto/expr.proto",
    //     "../../proto/action.proto",
    // ];
    // tonic_build::configure()
    //     .build_client(false)
    //     .build_server(false)
    //     .compile(protos, &["../../proto/"])?;
    //
    // for proto in protos {
    //     println!("cargo:rerun-if-changed={proto}");
    // }
    Ok(())
}
