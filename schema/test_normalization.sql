-- Test cases for message normalization function
-- Run these queries in ClickHouse to verify normalization behavior

USE heimsight;

-- Test 1: Timestamp normalization (ISO format with milliseconds)
SELECT normalizeLogMessage('Error at 2024-12-09T10:15:23.456Z') AS normalized;
-- Expected: 'Error at <TIMESTAMP>'
-- Result: Error at <TIMESTAMP> ✓

-- Test 2: UUID normalization
SELECT normalizeLogMessage('Request 550e8400-e29b-41d4-a716-446655440000 failed') AS normalized;
-- Expected: 'Request <UUID> failed'

-- Test 3: IP address normalization (IPv4)
SELECT normalizeLogMessage('Connection to 192.168.1.1:5432 timeout') AS normalized;
-- Expected: 'Connection to <IP>:<NUM> timeout'

-- Test 4: IP address normalization (IPv6)
SELECT normalizeLogMessage('Connected to 2001:0db8:85a3:0000:0000:8a2e:0370:7334') AS normalized;
-- Expected: 'Connected to <IPv6>'

-- Test 5: Number normalization (integers and floats)
SELECT normalizeLogMessage('Processed 12345 records in 3.14159 seconds') AS normalized;
-- Expected: 'Processed <NUM> records in <NUM> seconds'
-- Result: Processed <NUM> records in <NUM> seconds ✓

-- Test 5b: Verify floats are replaced correctly (not as separate integers)
SELECT normalizeLogMessage('Temperature: 98.6 degrees, Pressure: 1013.25 hPa') AS normalized;
-- Expected: 'Temperature: <NUM> degrees, Pressure: <NUM> hPa'
-- Result: Temperature: <NUM> degrees, Pressure: <NUM> hPa ✓

-- Test 5c: Multiple timestamps in one message
SELECT normalizeLogMessage('Started at 2024-12-09T10:15:23Z ended at 2024-12-09T11:30:45Z') AS normalized;
-- Expected: 'Started at <TIMESTAMP> ended at <TIMESTAMP>'
-- Result: Started at <TIMESTAMP> ended at <TIMESTAMP> ✓

-- Test 5d: Numbers with units (ms, s, MB, KB, etc)
SELECT normalizeLogMessage('Query took 5.01ms') AS normalized;
-- Expected: 'Query took <NUM>ms'
-- Result: Query took <NUM>ms ✓

SELECT normalizeLogMessage('Downloaded 250MB in 3.5s') AS normalized;
-- Expected: 'Downloaded <NUM>MB in <NUM>s'
-- Result: Downloaded <NUM>MB in <NUM>s ✓

SELECT normalizeLogMessage('Duration: 125.5ms, size: 1024KB') AS normalized;
-- Expected: 'Duration: <NUM>ms, size: <NUM>KB'
-- Result: Duration: <NUM>ms, size: <NUM>KB ✓

-- Test 6: URL normalization
SELECT normalizeLogMessage('GET https://api.example.com/users/123/posts returned 404') AS normalized;
-- Expected: 'GET <URL> returned <NUM>'

-- Test 7: Email normalization
SELECT normalizeLogMessage('User user@example.com logged in successfully') AS normalized;
-- Expected: 'User <EMAIL> logged in successfully'

-- Test 8: File path normalization
SELECT normalizeLogMessage('Failed to read /var/log/app/error.log') AS normalized;
-- Expected: 'Failed to read <PATH>'

-- Test 9: Hex value normalization
SELECT normalizeLogMessage('Memory address 0x7fff5fbff710 corrupted') AS normalized;
-- Expected: 'Memory address <HEX> corrupted'

-- Test 10: Complex message with multiple patterns
SELECT normalizeLogMessage(
    'Error at 2024-12-09T10:15:23Z: User user@example.com failed to connect to 192.168.1.1 (request_id: 12345, trace: 550e8400-e29b-41d4-a716-446655440000)'
) AS normalized;
-- Expected: 'Error at <TIMESTAMP>: User <EMAIL> failed to connect to <IP> (request_id: <NUM>, trace: <UUID>)'

-- Test 11: Verify normalization grouping
-- This simulates what happens in the materialized view
SELECT
    normalizeLogMessage(message) AS pattern,
    count() AS occurrences,
    any(message) AS example
FROM (
    SELECT 'Error at 2024-12-09T10:15:23Z: Connection failed' AS message
    UNION ALL
    SELECT 'Error at 2024-12-09T10:30:45Z: Connection failed'
    UNION ALL
    SELECT 'Error at 2024-12-09T11:00:12Z: Connection failed'
    UNION ALL
    SELECT 'Warning: Low memory'
)
GROUP BY pattern
ORDER BY occurrences DESC;
-- Expected: 'Error at <TIMESTAMP>: Connection failed' with count=3
--           'Warning: Low memory' with count=1

-- Test 12: Real-world application error examples
SELECT normalizeLogMessage('java.lang.NullPointerException at line 123 in file /app/src/Main.java') AS normalized;
-- Expected: 'java.lang.NullPointerException at line <NUM> in file <PATH>'

SELECT normalizeLogMessage('Database connection timeout after 30000ms to db-server-1.example.com:5432') AS normalized;
-- Expected: 'Database connection timeout after <NUM>ms to db-server-<NUM>.example.com:<NUM>'

SELECT normalizeLogMessage('HTTP 500 Internal Server Error: /api/v1/users/42/profile returned in 1250ms') AS normalized;
-- Expected: 'HTTP <NUM> Internal Server Error: <PATH> returned in <NUM>ms'

-- Test 13: Verify normalized_message is automatically computed in logs table
-- (This test only works after inserting actual data)
-- SELECT message, normalized_message FROM logs LIMIT 5;

