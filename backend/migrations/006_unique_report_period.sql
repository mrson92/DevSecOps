-- De-duplicate any existing reports that share the same type + period start,
-- keeping the newest one (by generated_at, rowid as tiebreaker), before adding
-- the unique constraint. This handles databases that accumulated duplicates
-- before this constraint existed. Safe to run on a clean database (no-op).
DELETE FROM reports
WHERE id NOT IN (
    SELECT id FROM (
        SELECT id,
               ROW_NUMBER() OVER (PARTITION BY type, period_start
                                  ORDER BY generated_at DESC, rowid DESC) AS rn
        FROM reports
    ) WHERE rn = 1
);

-- Unique report period: same type + start of period must be unique
CREATE UNIQUE INDEX IF NOT EXISTS idx_reports_type_period_start
    ON reports(type, period_start);
