//! Build script for compiling OTLP protobuf definitions.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Tell cargo to rerun this build script if proto files change
    println!("cargo:rerun-if-changed=proto/");

    // Compile OTLP protobuf definitions
    tonic_build::configure()
        .build_server(false) // We only need client types for now
        .emit_rerun_if_changed(false) // Don't rebuild unless proto files change
        .compile_protos(
            &[
                "proto/opentelemetry/proto/common/v1/common.proto",
                "proto/opentelemetry/proto/resource/v1/resource.proto",
                "proto/opentelemetry/proto/logs/v1/logs.proto",
                "proto/opentelemetry/proto/metrics/v1/metrics.proto",
                "proto/opentelemetry/proto/trace/v1/trace.proto",
                "proto/opentelemetry/proto/collector/logs/v1/logs_service.proto",
                "proto/opentelemetry/proto/collector/metrics/v1/metrics_service.proto",
                "proto/opentelemetry/proto/collector/trace/v1/trace_service.proto",
            ],
            &["proto"],
        )?;

    Ok(())
}
