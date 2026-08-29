-- Add Sigma-style tags to rules (e.g. attack.t1110, status.experimental, cve.2021-xxxx)
ALTER TABLE rules ADD COLUMN tags TEXT DEFAULT '[]';

-- Backfill tags from existing MITRE techniques for reference convenience.
-- This derives metadata (identifiers) only; it does not import any Sigma rule content.
UPDATE rules
SET tags = (
    SELECT '["' || group_concat(
        'attack.t' || substr(trim(value), 2),
        '","'
    ) || '"]'
    FROM json_each(rules.mitre_techniques)
    WHERE trim(value) LIKE 'T%'
)
WHERE mitre_techniques IS NOT NULL
  AND mitre_techniques != '[]'
  AND json_valid(mitre_techniques);
