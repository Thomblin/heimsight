# Updating the `normalized_message` Column

If you need to update the `normalizeLogMessage()` function and apply it to existing data in the `logs` table, follow these steps:

## Why This Is Needed

The `normalized_message` column is a `MATERIALIZED` column that is computed on insert. If you update the `normalizeLogMessage()` function, existing rows will still have the old normalization until the column is recreated.

## Update Process

```sql
USE heimsight;

-- Step 1: Drop materialized views that reference normalized_message
DROP VIEW IF EXISTS logs_1hour_counts_mv;
DROP VIEW IF EXISTS logs_1day_counts_mv;

-- Step 2: Drop the old normalized_message column
ALTER TABLE logs DROP COLUMN IF EXISTS normalized_message;

-- Step 3: Add the column back with the updated function
ALTER TABLE logs ADD COLUMN normalized_message String MATERIALIZED normalizeLogMessage(message) AFTER message;

-- Step 4: Add the index back
ALTER TABLE logs ADD INDEX idx_normalized normalized_message TYPE tokenbf_v1(32768, 3, 0) GRANULARITY 1;

-- Step 5: Recreate the hourly materialized view
CREATE MATERIALIZED VIEW logs_1hour_counts_mv TO logs_1hour_counts AS
SELECT
    toStartOfHour(toDateTime(timestamp / 1000000000)) AS timestamp,
    level,
    service,
    normalized_message,
    count() AS count,
    any(message) AS sample_message
FROM logs
GROUP BY timestamp, level, service, normalized_message;

-- Step 6: Recreate the daily materialized view
CREATE MATERIALIZED VIEW logs_1day_counts_mv TO logs_1day_counts AS
SELECT
    toStartOfDay(toDateTime(timestamp / 1000000000)) AS timestamp,
    level,
    service,
    normalized_message,
    count() AS count,
    any(message) AS sample_message
FROM logs
GROUP BY timestamp, level, service, normalized_message;
```

## Using the Makefile

Or simply use the provided script:

```bash
# This will update the function and recreate the column
docker compose exec -T clickhouse clickhouse-client -d heimsight --multiquery < schema/00_functions.sql

# Then run the update script above
```

## Important Notes

1. **Materialized Views**: Must be dropped before modifying the column they reference
2. **Indexes**: Are automatically dropped when the column is dropped
3. **Existing Data**: Will be recomputed with the new function after the column is recreated
4. **New Inserts**: After recreation, new data from materialized views will use the updated normalization

## Verification

Check that normalization is working correctly:

```sql
-- Test on existing data
SELECT message, normalized_message 
FROM logs 
WHERE message LIKE '%2024%' 
LIMIT 5;

-- Check aggregation tables
SELECT normalized_message, count() 
FROM logs_1hour_counts 
GROUP BY normalized_message 
ORDER BY count() DESC 
LIMIT 10;
```

## Common Issues

### Error: "Cannot drop column referenced by materialized view"
**Solution**: Drop the materialized views first (Step 1)

### Error: "Wrong index name"
**Solution**: The index is automatically dropped with the column, skip the index drop step

### Old normalized values still showing
**Solution**: Run `OPTIMIZE TABLE logs FINAL;` to force recomputation
