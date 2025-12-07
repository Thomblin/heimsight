//! OpenTelemetry Protocol (OTLP) support.
//!
//! This module provides conversion between OTLP protobuf types and internal Heimsight types.
//!
//! # Example
//!
//! ```ignore
//! use shared::otlp::conversions::otlp_log_to_log_entry;
//! // Convert OTLP logs to internal LogEntry format
//! ```

pub mod conversions;

// Include the generated protobuf code
pub mod proto {
    //! Generated protobuf types from OTLP definitions.

    pub mod common {
        //! Common OTLP types.
        pub mod v1 {
            #![allow(clippy::all)]
            #![allow(missing_docs)]
            tonic::include_proto!("opentelemetry.proto.common.v1");
        }
    }

    pub mod resource {
        //! Resource types.
        pub mod v1 {
            #![allow(clippy::all)]
            #![allow(missing_docs)]
            tonic::include_proto!("opentelemetry.proto.resource.v1");
        }
    }

    pub mod logs {
        //! Log types.
        pub mod v1 {
            #![allow(clippy::all)]
            #![allow(missing_docs)]
            tonic::include_proto!("opentelemetry.proto.logs.v1");
        }
    }

    pub mod metrics {
        //! Metric types.
        pub mod v1 {
            #![allow(clippy::all)]
            #![allow(missing_docs)]
            tonic::include_proto!("opentelemetry.proto.metrics.v1");
        }
    }

    pub mod trace {
        //! Trace types.
        pub mod v1 {
            #![allow(clippy::all)]
            #![allow(missing_docs)]
            tonic::include_proto!("opentelemetry.proto.trace.v1");
        }
    }

    pub mod collector {
        //! Collector service types.

        pub mod logs {
            //! Log collector service.
            pub mod v1 {
                #![allow(clippy::all)]
                #![allow(missing_docs)]
                tonic::include_proto!("opentelemetry.proto.collector.logs.v1");
            }
        }

        pub mod metrics {
            //! Metrics collector service.
            pub mod v1 {
                #![allow(clippy::all)]
                #![allow(missing_docs)]
                tonic::include_proto!("opentelemetry.proto.collector.metrics.v1");
            }
        }

        pub mod trace {
            //! Trace collector service.
            pub mod v1 {
                #![allow(clippy::all)]
                #![allow(missing_docs)]
                tonic::include_proto!("opentelemetry.proto.collector.trace.v1");
            }
        }
    }
}
