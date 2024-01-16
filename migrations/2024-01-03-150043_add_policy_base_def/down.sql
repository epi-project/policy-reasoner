-- This file should undo anything in `up.sql`
ALTER TABLE policies
  DROP COLUMN reasoner_connector_context;