#!/bin/bash
# Test log normalization, span aggregation, and overall aggregation features
# This script sends various test logs and spans to demonstrate:
#   - Log message normalization
#   - Span performance aggregation
#   - Verify aggregation is working correctly
#
# Usage: ./scripts/test-normalization.sh [quick|full|verify]
#   quick - Send minimal test data (default)
#   full  - Send comprehensive test data with various patterns
#   verify - Query aggregated data to verify it's working
#
# Environment variables:
#   HEIMSIGHT_HOST - Host to send data to (default: localhost)
#   HEIMSIGHT_PORT - Port to send data to (default: 8080)

set -e

HEIMSIGHT_HOST="${HEIMSIGHT_HOST:-localhost}"
HEIMSIGHT_PORT="${HEIMSIGHT_PORT:-8080}"
BASE_URL="http://${HEIMSIGHT_HOST}:${HEIMSIGHT_PORT}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_header() {
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}========================================${NC}"
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_info() {
    echo -e "${YELLOW}ℹ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

# Function to send a log entry
send_log() {
    local message="$1"
    local level="${2:-info}"
    local service="${3:-test-service}"
    local attributes="${4:-{}}"

    curl -s -X POST "${BASE_URL}/api/v1/logs" \
        -H "Content-Type: application/json" \
        -d "{
            \"message\": \"${message}\",
            \"service\": \"${service}\",
            \"level\": \"${level}\",
            \"attributes\": ${attributes}
        }" > /dev/null
}

# Function to send a batch of logs
send_batch() {
    local json="$1"

    curl -s -X POST "${BASE_URL}/api/v1/logs" \
        -H "Content-Type: application/json" \
        -d "${json}" > /dev/null
}

# Function to send a span
send_span() {
    local trace_id="$1"
    local span_id="$2"
    local operation="$3"
    local duration_ms="${4:-100}"
    local status="${5:-OK}"
    local service="${6:-test-service}"
    local span_kind="${7:-INTERNAL}"

    local duration_ns=$((duration_ms * 1000000))
    local now_ns=$(date +%s%N)
    local start_ns=$((now_ns - duration_ns))

    curl -s -X POST "${BASE_URL}/api/v1/traces" \
        -H "Content-Type: application/json" \
        -d "{
            \"trace_id\": \"${trace_id}\",
            \"span_id\": \"${span_id}\",
            \"name\": \"${operation}\",
            \"operation\": \"${operation}\",
            \"start_time\": ${start_ns},
            \"end_time\": ${now_ns},
            \"duration_ns\": ${duration_ns},
            \"service\": \"${service}\",
            \"span_kind\": \"${span_kind}\",
            \"status_code\": \"${status}\"
        }" > /dev/null
}

# Function to generate a random trace ID
generate_trace_id() {
    echo "$(uuidgen | tr -d '-')"
}

# Function to generate a random span ID
generate_span_id() {
    echo "$(openssl rand -hex 8 2>/dev/null || echo "$(date +%s%N | md5sum | cut -c1-16)")"
}

# Quick test - minimal test data to verify normalization and aggregation
test_quick() {
    print_header "Quick Test: Logs & Spans"

    # Log tests
    print_info "Sending logs with timestamps (should normalize to <TIMESTAMP>)..."
    send_log "Error at 2024-12-09T10:15:23.456Z" "error" "api"
    send_log "Error at 2024-12-09T11:30:45.789Z" "error" "api"
    send_log "Error at 2024-12-10T08:22:11.123Z" "error" "api"
    print_success "Sent 3 logs with different timestamps"

    print_info "Sending logs with IP addresses (should normalize to <IP>)..."
    send_log "Connection failed to 192.168.1.100" "error" "network"
    send_log "Connection failed to 10.0.0.25" "error" "network"
    send_log "Connection failed to 172.16.5.99" "error" "network"
    print_success "Sent 3 logs with different IPs"

    print_info "Sending logs with user IDs (should normalize to <NUM>)..."
    send_log "User 12345 logged in" "info" "auth"
    send_log "User 67890 logged in" "info" "auth"
    send_log "User 99999 logged in" "info" "auth"
    print_success "Sent 3 logs with different user IDs"

    # Span tests
    echo ""
    print_info "Sending spans with varying latencies..."
    local trace_id=$(generate_trace_id)
    for i in {1..5}; do
        local span_id=$(generate_span_id)
        local duration=$((50 + i * 20))  # 70ms, 90ms, 110ms, 130ms, 150ms
        send_span "${trace_id}" "${span_id}" "GET /api/users" "${duration}" "OK" "api" "SERVER"
    done
    print_success "Sent 5 spans with different latencies (70-150ms)"

    print_info "Sending spans with errors..."
    trace_id=$(generate_trace_id)
    for i in {1..3}; do
        local span_id=$(generate_span_id)
        send_span "${trace_id}" "${span_id}" "POST /api/data" "200" "ERROR" "api" "SERVER"
    done
    print_success "Sent 3 error spans"

    echo ""
    print_success "Quick test complete!"
    echo ""
    print_info "Log patterns grouped:"
    echo "  1. 'Error at <TIMESTAMP>' (3 occurrences)"
    echo "  2. 'Connection failed to <IP>' (3 occurrences)"
    echo "  3. 'User <NUM> logged in' (3 occurrences)"
    echo ""
    print_info "Span aggregations:"
    echo "  - 5 successful spans for 'GET /api/users' (varying latency)"
    echo "  - 3 error spans for 'POST /api/data'"
    echo ""
    print_info "Run with 'verify' argument to check aggregation tables"
}

# Full test - comprehensive test data with various normalization patterns
test_full() {
    print_header "Full Normalization Test Suite"

    # Test 1: Timestamps
    print_info "Test 1: ISO Timestamps..."
    send_batch '[
        {"message": "Event at 2024-12-09T10:15:23.456Z", "service": "api", "level": "info"},
        {"message": "Event at 2024-12-09T11:30:45.789Z", "service": "api", "level": "info"},
        {"message": "Event at 2024-12-10T08:22:11.123Z", "service": "api", "level": "info"},
        {"message": "Event at 2024-12-10 14:35:22", "service": "api", "level": "info"}
    ]'
    print_success "Sent 4 logs with timestamps"

    # Test 2: UUIDs
    print_info "Test 2: UUIDs..."
    send_batch '[
        {"message": "Processing request 550e8400-e29b-41d4-a716-446655440000", "service": "api", "level": "debug"},
        {"message": "Processing request a1b2c3d4-e5f6-7890-abcd-ef1234567890", "service": "api", "level": "debug"},
        {"message": "Processing request 12345678-1234-1234-1234-123456789012", "service": "api", "level": "debug"}
    ]'
    print_success "Sent 3 logs with UUIDs"

    # Test 3: IP Addresses (IPv4)
    print_info "Test 3: IPv4 Addresses..."
    send_batch '[
        {"message": "Request from 192.168.1.100", "service": "gateway", "level": "info"},
        {"message": "Request from 10.0.0.25", "service": "gateway", "level": "info"},
        {"message": "Request from 172.16.5.99", "service": "gateway", "level": "info"},
        {"message": "Request from 8.8.8.8", "service": "gateway", "level": "info"}
    ]'
    print_success "Sent 4 logs with IPv4 addresses"

    # Test 4: Numbers (integers and floats)
    print_info "Test 4: Numbers..."
    send_batch '[
        {"message": "Query took 45.23ms", "service": "database", "level": "debug"},
        {"message": "Query took 123.89ms", "service": "database", "level": "debug"},
        {"message": "Query took 5.01ms", "service": "database", "level": "debug"},
        {"message": "Processed 12345 records", "service": "database", "level": "info"},
        {"message": "Processed 67890 records", "service": "database", "level": "info"}
    ]'
    print_success "Sent 5 logs with numbers"

    # Test 5: URLs
    print_info "Test 5: URLs..."
    send_batch '[
        {"message": "Fetched https://api.example.com/users/123", "service": "api", "level": "debug"},
        {"message": "Fetched https://api.example.com/users/456", "service": "api", "level": "debug"},
        {"message": "Fetched http://internal.service/data?id=789", "service": "api", "level": "debug"}
    ]'
    print_success "Sent 3 logs with URLs"

    # Test 6: Email addresses
    print_info "Test 6: Email Addresses..."
    send_batch '[
        {"message": "Password reset for user@example.com", "service": "auth", "level": "info"},
        {"message": "Password reset for admin@test.org", "service": "auth", "level": "info"},
        {"message": "Password reset for john.doe@company.net", "service": "auth", "level": "info"}
    ]'
    print_success "Sent 3 logs with emails"

    # Test 7: File paths
    print_info "Test 7: File Paths..."
    send_batch '[
        {"message": "Loading config from /etc/app/config.yaml", "service": "config", "level": "info"},
        {"message": "Loading config from /home/user/.config/app.yaml", "service": "config", "level": "info"},
        {"message": "Loading config from ./config/local.yaml", "service": "config", "level": "info"}
    ]'
    print_success "Sent 3 logs with file paths"

    # Test 8: Hex values
    print_info "Test 8: Hex Values..."
    send_batch '[
        {"message": "Memory address 0x1a2b3c4d", "service": "system", "level": "debug"},
        {"message": "Memory address 0xdeadbeef", "service": "system", "level": "debug"},
        {"message": "Memory address 0x7fff0000", "service": "system", "level": "debug"}
    ]'
    print_success "Sent 3 logs with hex values"

    # Test 9: Complex mixed patterns
    print_info "Test 9: Complex Mixed Patterns..."
    send_batch '[
        {"message": "User user123@example.com from 192.168.1.100 accessed /api/data at 2024-12-09T10:15:23Z", "service": "audit", "level": "info"},
        {"message": "User admin@test.org from 10.0.0.25 accessed /api/data at 2024-12-09T11:30:45Z", "service": "audit", "level": "info"},
        {"message": "User john@company.net from 172.16.5.99 accessed /api/data at 2024-12-10T08:22:11Z", "service": "audit", "level": "info"}
    ]'
    print_success "Sent 3 logs with mixed patterns"

    # Test 10: Error patterns for aggregation
    print_info "Test 10: Repeated Error Patterns (for aggregation)..."
    for i in {1..10}; do
        send_log "Database connection timeout after ${i}000ms" "error" "database"
    done
    print_success "Sent 10 logs with database errors"

    for i in {1..15}; do
        send_log "HTTP 404 Not Found: /api/users/${i}" "warn" "api"
    done
    print_success "Sent 15 logs with 404 errors"

    for i in {1..20}; do
        send_log "Failed to authenticate user id=${i}" "error" "auth"
    done
    print_success "Sent 20 logs with auth errors"

    # Test 11: Span latency patterns
    echo ""
    print_info "Test 11: Span Latency Patterns..."
    local trace_id=$(generate_trace_id)
    
    # Fast operations
    for i in {1..20}; do
        local span_id=$(generate_span_id)
        local duration=$((10 + RANDOM % 40))  # 10-50ms
        send_span "${trace_id}" "${span_id}" "GET /api/health" "${duration}" "OK" "api" "SERVER"
    done
    print_success "Sent 20 fast spans (10-50ms)"

    # Medium operations
    for i in {1..15}; do
        local span_id=$(generate_span_id)
        local duration=$((50 + RANDOM % 100))  # 50-150ms
        send_span "${trace_id}" "${span_id}" "GET /api/users" "${duration}" "OK" "api" "SERVER"
    done
    print_success "Sent 15 medium spans (50-150ms)"

    # Slow operations
    for i in {1..10}; do
        local span_id=$(generate_span_id)
        local duration=$((200 + RANDOM % 300))  # 200-500ms
        send_span "${trace_id}" "${span_id}" "POST /api/search" "${duration}" "OK" "api" "SERVER"
    done
    print_success "Sent 10 slow spans (200-500ms)"

    # Test 12: Span error patterns
    print_info "Test 12: Span Error Patterns..."
    
    # Database timeout errors
    for i in {1..8}; do
        local span_id=$(generate_span_id)
        send_span "${trace_id}" "${span_id}" "db.query" "5000" "ERROR" "database" "CLIENT"
    done
    print_success "Sent 8 database error spans"

    # Auth failures
    for i in {1..5}; do
        local span_id=$(generate_span_id)
        send_span "${trace_id}" "${span_id}" "auth.verify" "100" "ERROR" "auth" "INTERNAL"
    done
    print_success "Sent 5 auth error spans"

    # Test 13: Multi-span traces
    print_info "Test 13: Complex Multi-Span Traces..."
    for i in {1..5}; do
        local trace=$(generate_trace_id)
        local root_span=$(generate_span_id)
        
        # Root span
        send_span "${trace}" "${root_span}" "HTTP GET /api/users/123" "250" "OK" "gateway" "SERVER"
        
        # Child spans
        send_span "${trace}" "$(generate_span_id)" "auth.check" "20" "OK" "auth" "INTERNAL"
        send_span "${trace}" "$(generate_span_id)" "db.query" "150" "OK" "database" "CLIENT"
        send_span "${trace}" "$(generate_span_id)" "cache.get" "10" "OK" "cache" "CLIENT"
    done
    print_success "Sent 5 multi-span traces (4 spans each)"

    echo ""
    print_success "Full test suite complete!"
    echo ""
    print_info "Total data sent:"
    echo "  - ~80 logs across 10 test categories"
    echo "  - ~88 spans across 3 test categories"
    echo "  - 5 multi-span traces (20 spans total)"
    print_info "Run with 'verify' argument to check aggregation tables"
}

# Verify aggregation is working
test_verify() {
    print_header "Verifying Log & Span Aggregation"

    # ============================================================================
    # LOG AGGREGATION VERIFICATION
    # ============================================================================
    print_header "Log Aggregation Verification"

    print_info "Querying raw logs table..."
    echo ""
    echo "Sample raw logs:"
    curl -s "${BASE_URL}/api/v1/logs?limit=5" | jq -r '.[] | "  [\(.level)] \(.message)"' 2>/dev/null || print_error "Failed to query logs (jq may not be installed)"

    echo ""
    print_info "Checking normalized messages in raw logs..."
    echo ""
    echo "Run this query in ClickHouse to see normalized messages:"
    echo "  SELECT message, normalized_message, count() as cnt"
    echo "  FROM logs"
    echo "  GROUP BY message, normalized_message"
    echo "  ORDER BY cnt DESC"
    echo "  LIMIT 10;"

    echo ""
    print_info "Checking hourly log aggregations..."
    echo ""
    echo "Run this query to see aggregated log counts:"
    echo "  SELECT"
    echo "    timestamp,"
    echo "    service,"
    echo "    level,"
    echo "    normalized_message,"
    echo "    count,"
    echo "    sample_message"
    echo "  FROM logs_1hour_counts"
    echo "  ORDER BY count DESC"
    echo "  LIMIT 20;"

    echo ""
    print_info "Checking daily log aggregations..."
    echo ""
    echo "Run this query to see daily aggregations:"
    echo "  SELECT"
    echo "    timestamp,"
    echo "    service,"
    echo "    normalized_message,"
    echo "    sum(count) as total_count,"
    echo "    any(sample_message) as example"
    echo "  FROM logs_1day_counts"
    echo "  GROUP BY timestamp, service, normalized_message"
    echo "  ORDER BY total_count DESC"
    echo "  LIMIT 20;"

    echo ""
    print_info "Finding most common error patterns..."
    echo ""
    echo "Run this query to find common error patterns:"
    echo "  SELECT"
    echo "    normalized_message,"
    echo "    sum(count) as occurrences,"
    echo "    any(sample_message) as example"
    echo "  FROM logs_1hour_counts"
    echo "  WHERE level = 'error'"
    echo "  GROUP BY normalized_message"
    echo "  ORDER BY occurrences DESC"
    echo "  LIMIT 10;"

    # ============================================================================
    # SPAN AGGREGATION VERIFICATION
    # ============================================================================
    echo ""
    print_header "Span Aggregation Verification"

    print_info "Querying raw spans table..."
    echo ""
    echo "Sample raw spans:"
    curl -s "${BASE_URL}/api/v1/traces?limit=5" | jq -r '.[] | "  [\(.service)] \(.operation) - \(.duration_ns / 1000000)ms"' 2>/dev/null || print_error "Failed to query spans (jq may not be installed)"

    echo ""
    print_info "Checking hourly span performance statistics..."
    echo ""
    echo "Run this query to see span latency aggregations:"
    echo "  SELECT"
    echo "    timestamp,"
    echo "    service,"
    echo "    operation,"
    echo "    span_count,"
    echo "    avg_duration_ns / 1000000 as avg_ms,"
    echo "    p50_duration_ns / 1000000 as p50_ms,"
    echo "    p95_duration_ns / 1000000 as p95_ms,"
    echo "    p99_duration_ns / 1000000 as p99_ms"
    echo "  FROM spans_1hour_stats"
    echo "  ORDER BY span_count DESC"
    echo "  LIMIT 20;"

    echo ""
    print_info "Finding slowest operations (P99 latency)..."
    echo ""
    echo "Run this query to find slow operations:"
    echo "  SELECT"
    echo "    service,"
    echo "    operation,"
    echo "    avg(p99_duration_ns) / 1000000 as p99_ms,"
    echo "    sum(span_count) as total_spans"
    echo "  FROM spans_1day_stats"
    echo "  GROUP BY service, operation"
    echo "  ORDER BY p99_ms DESC"
    echo "  LIMIT 10;"

    echo ""
    print_info "Checking error rates by operation..."
    echo ""
    echo "Run this query to see error rates:"
    echo "  SELECT"
    echo "    service,"
    echo "    operation,"
    echo "    countIf(status_code != 'OK') as error_count,"
    echo "    sum(span_count) as total_count,"
    echo "    (error_count * 100.0 / total_count) as error_rate_pct"
    echo "  FROM spans_1hour_stats"
    echo "  GROUP BY service, operation"
    echo "  HAVING error_rate_pct > 0"
    echo "  ORDER BY error_rate_pct DESC"
    echo "  LIMIT 10;"

    echo ""
    print_info "Checking trace statistics..."
    echo ""
    echo "Run this query to see trace characteristics:"
    echo "  SELECT"
    echo "    timestamp,"
    echo "    service,"
    echo "    unique_traces,"
    echo "    total_spans,"
    echo "    total_spans / unique_traces as avg_spans_per_trace"
    echo "  FROM traces_1hour_stats"
    echo "  ORDER BY timestamp DESC"
    echo "  LIMIT 20;"

    echo ""
    print_info "Finding services with most complex traces..."
    echo ""
    echo "Run this query to see trace complexity:"
    echo "  SELECT"
    echo "    service,"
    echo "    sum(unique_traces) as total_traces,"
    echo "    sum(total_spans) / sum(unique_traces) as avg_complexity"
    echo "  FROM traces_1day_stats"
    echo "  GROUP BY service"
    echo "  ORDER BY avg_complexity DESC;"

    echo ""
    print_success "Verification queries provided!"
    echo ""
    print_info "Note: Aggregation tables are populated by materialized views."
    print_info "      Data may take a few seconds to appear in aggregated tables."
    print_info "      Log normalization is applied automatically during insertion."
    print_info "      Span statistics include P50, P95, P99 latency percentiles."
}

# Show usage
show_usage() {
    echo "Usage: $0 [quick|full|verify]"
    echo ""
    echo "Commands:"
    echo "  quick  - Send minimal test data (default)"
    echo "  full   - Send comprehensive test data with various patterns"
    echo "  verify - Show queries to verify aggregation is working"
    echo ""
    echo "Environment variables:"
    echo "  HEIMSIGHT_HOST - Host to send data to (default: localhost)"
    echo "  HEIMSIGHT_PORT - Port to send data to (default: 8080)"
    echo ""
    echo "Examples:"
    echo "  $0 quick"
    echo "  $0 full"
    echo "  $0 verify"
    echo "  HEIMSIGHT_HOST=192.168.1.100 $0 full"
}

# Main execution
case "${1:-quick}" in
    quick)
        test_quick
        ;;
    full)
        test_full
        ;;
    verify)
        test_verify
        ;;
    help|-h|--help)
        show_usage
        ;;
    *)
        print_error "Unknown command: $1"
        echo ""
        show_usage
        exit 1
        ;;
esac
