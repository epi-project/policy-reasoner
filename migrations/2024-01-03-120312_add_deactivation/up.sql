-- Your SQL goes here
ALTER TABLE active_version
  ADD deactivated_on DATETIME NULL;

ALTER TABLE active_version
  ADD deactivated_by TEXT NULL;
