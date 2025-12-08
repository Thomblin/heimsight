//! Build script for compiling OTLP protobuf definitions.

use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Tell cargo to rerun this build script if proto files change
    println!("cargo:rerun-if-changed=proto/");

    let proto_files = &[
        "proto/opentelemetry/proto/common/v1/common.proto",
        "proto/opentelemetry/proto/resource/v1/resource.proto",
        "proto/opentelemetry/proto/logs/v1/logs.proto",
        "proto/opentelemetry/proto/metrics/v1/metrics.proto",
        "proto/opentelemetry/proto/trace/v1/trace.proto",
        "proto/opentelemetry/proto/collector/logs/v1/logs_service.proto",
        "proto/opentelemetry/proto/collector/metrics/v1/metrics_service.proto",
        "proto/opentelemetry/proto/collector/trace/v1/trace_service.proto",
    ];

    let proto_include_dirs = &["proto"];

    // Get output directory
    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);

    // Create a file descriptor set for pbjson
    let descriptor_path = out_dir.join("proto_descriptor.bin");

    // Compile OTLP protobuf definitions with file descriptor output
    tonic_build::configure()
        .build_server(true) // Enable gRPC server generation
        .emit_rerun_if_changed(false) // Don't rebuild unless proto files change
        .file_descriptor_set_path(&descriptor_path)
        .compile_protos(proto_files, proto_include_dirs)?;

    // Generate serde implementations using pbjson
    let descriptor_set = std::fs::read(&descriptor_path)?;
    pbjson_build::Builder::new()
        .register_descriptors(&descriptor_set)?
        .build(&[".opentelemetry"])?;

    Ok(())
}
