-- This file should undo anything in `up.sql`
ALTER TABLE active_version
  DROP COLUMN deactivated_on DATETIME NULL;

ALTER TABLE active_version
  DROP COLUMN deactivated_by TEXT NULL;