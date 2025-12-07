#!/bin/bash
# Test OTLP endpoints with telemetrygen
# Usage: ./scripts/test-otlp.sh [traces|logs|metrics|all]

set -e

HEIMSIGHT_HOST="${HEIMSIGHT_HOST:-localhost}"
HEIMSIGHT_PORT="${HEIMSIGHT_PORT:-8080}"
ENDPOINT="${HEIMSIGHT_HOST}:${HEIMSIGHT_PORT}"
COUNT="${COUNT:-10}"
SERVICE="${SERVICE:-test-service}"

IMAGE="ghcr.io/open-telemetry/opentelemetry-collector-contrib/telemetrygen:latest"

# Use host networking on Linux, or host.docker.internal on Mac/Windows
if [[ "$OSTYPE" == "darwin"* ]] || [[ "$OSTYPE" == "msys"* ]] || [[ "$OSTYPE" == "win32"* ]]; then
    NETWORK_OPTS=""
    ENDPOINT="http://host.docker.internal:${HEIMSIGHT_PORT}"
else
    NETWORK_OPTS="--network=host"
fi

send_traces() {
    echo "Sending ${COUNT} traces to ${ENDPOINT}..."
    docker run --rm ${NETWORK_OPTS} ${IMAGE} \
        traces \
        --otlp-http \
        --otlp-endpoint="${ENDPOINT}" \
        --otlp-insecure \
        --traces="${COUNT}" \
        --service="${SERVICE}"
    echo "Done! Check with: curl ${ENDPOINT}/api/v1/traces"
}

send_logs() {
    echo "Sending ${COUNT} logs to ${ENDPOINT}..."
    docker run --rm ${NETWORK_OPTS} ${IMAGE} \
        logs \
        --otlp-http \
        --otlp-endpoint="${ENDPOINT}" \
        --otlp-insecure \
        --logs="${COUNT}" \
        --service="${SERVICE}"
    echo "Done! Check with: curl ${ENDPOINT}/api/v1/logs"
}

send_metrics() {
    echo "Sending ${COUNT} metrics to ${ENDPOINT}..."
    docker run --rm ${NETWORK_OPTS} ${IMAGE} \
        metrics \
        --otlp-http \
        --otlp-endpoint="${ENDPOINT}" \
        --otlp-insecure \
        --metrics="${COUNT}" \
        --service="${SERVICE}"
    echo "Done! Check with: curl ${ENDPOINT}/api/v1/metrics"
}

case "${1:-all}" in
    traces)
        send_traces
        ;;
    logs)
        send_logs
        ;;
    metrics)
        send_metrics
        ;;
    all)
        send_traces
        echo ""
        send_logs
        echo ""
        send_metrics
        ;;
    *)
        echo "Usage: $0 [traces|logs|metrics|all]"
        echo ""
        echo "Environment variables:"
        echo "  HEIMSIGHT_HOST  - Host to send data to (default: localhost)"
        echo "  HEIMSIGHT_PORT  - Port to send data to (default: 8080)"
        echo "  COUNT           - Number of items to send (default: 10)"
        echo "  SERVICE         - Service name to use (default: test-service)"
        exit 1
        ;;
esac
