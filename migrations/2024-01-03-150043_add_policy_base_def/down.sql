-- This file should undo anything in `up.sql`
ALTER TABLE policies
  DROP COLUMN base_defs;