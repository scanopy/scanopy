-- Add scan_count to track how many scans have completed (for light/full scan cycling)
-- Add force_full_scan flag to allow users to override the next scan to be a full scan
ALTER TABLE discovery ADD COLUMN scan_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE discovery ADD COLUMN force_full_scan BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE discovery ADD COLUMN pending_credential_ids UUID[] NOT NULL DEFAULT '{}';
