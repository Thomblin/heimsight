-- User-defined functions for Heimsight
-- These functions are used across multiple tables and aggregations

USE heimsight;

-- Message normalization function for log aggregation
-- Strips variable parts (timestamps, UUIDs, numbers, IPs, etc.) to group similar messages
-- IMPORTANT: Nested replaceRegexpAll executes INNERMOST first, so specific patterns go at the bottom!
CREATE FUNCTION IF NOT EXISTS normalizeLogMessage AS (msg) -> (
    -- Step 10: Replace integers last (most generic, outermost)
    replaceRegexpAll(
        -- Step 9: Replace floating point numbers (before integers)
        replaceRegexpAll(
            -- Step 8: Replace file paths
            replaceRegexpAll(
                -- Step 7: Replace email addresses
                replaceRegexpAll(
                    -- Step 6: Replace URLs
                    replaceRegexpAll(
                        -- Step 5: Replace hex values
                        replaceRegexpAll(
                            -- Step 4: Replace IPv6 addresses
                            replaceRegexpAll(
                                -- Step 3: Replace IPv4 addresses
                                replaceRegexpAll(
                                    -- Step 2: Replace UUIDs
                                    replaceRegexpAll(
                                        -- Step 1: Replace ISO timestamps FIRST (innermost, executes first!)
                                        replaceRegexpAll(
                                            msg,
                                            -- ISO timestamps and common date formats
                                            '\\d{4}-\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}:\\d{2}(?:\\.\\d{3,9})?(?:Z|[+-]\\d{2}:\\d{2})?',
                                            '<TIMESTAMP>'
                                        ),
                                        -- UUID: 8-4-4-4-12 format
                                        '\\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\\b',
                                        '<UUID>'
                                    ),
                                    -- IPv4: xxx.xxx.xxx.xxx
                                    '\\b\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}\\b',
                                    '<IP>'
                                ),
                                -- IPv6: simplified pattern (groups of hex separated by colons)
                                '\\b(?:[0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}\\b',
                                '<IPv6>'
                            ),
                            -- Hex values: 0x followed by hex digits
                            '\\b0x[0-9a-fA-F]+\\b',
                            '<HEX>'
                        ),
                        -- URLs: http(s)://domain.com/path
                        'https?://[\\w./?=&-]+',
                        '<URL>'
                    ),
                    -- Email: user@domain.com
                    '\\b[\\w.-]+@[\\w.-]+\\.\\w{2,}\\b',
                    '<EMAIL>'
                ),
                -- Unix paths: /path/to/file or ./relative/path
                '(?:^|\\s)(\\./|/)[\\w./-]+',
                ' <PATH>'
            ),
            -- Floats: MUST be replaced BEFORE integers
            -- Matches floats even when followed by units (ms, s, MB, KB, etc)
            '\\b-?\\d+\\.\\d+',
            '<NUM>'
        ),
        -- Integers: capture sequences of digits (including negative)
        -- Matches integers even when followed by units
        '\\b-?\\d+',
        '<NUM>'
    )
);

-- Example usage:
-- SELECT normalizeLogMessage('Error at 2024-12-09T10:15:23.456Z: Connection to 192.168.1.1 failed') AS normalized;
-- Result: 'Error at <TIMESTAMP>: Connection to <IP> failed'
--
-- SELECT normalizeLogMessage('Query took 5.01ms and returned 250MB') AS normalized;
-- Result: 'Query took <NUM>ms and returned <NUM>MB'
--
-- SELECT normalizeLogMessage('User user@example.com logged in from /home/user/app') AS normalized;
-- Result: 'User <EMAIL> logged in from <PATH>'
--
-- SELECT normalizeLogMessage('Request 12345 to https://api.example.com/users/67890 returned 404') AS normalized;
-- Result: 'Request <NUM> to <URL> returned <NUM>'

