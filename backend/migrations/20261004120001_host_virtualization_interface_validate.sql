-- Validate `hosts_virtualization_interface_id_fkey`, added `NOT VALID` in 20261004120000. A
-- separate step, so the add takes no long lock; the column is new, so this checks only the rows
-- written since.

SET lock_timeout = '5s';
SET statement_timeout = '0';

ALTER TABLE hosts VALIDATE CONSTRAINT hosts_virtualization_interface_id_fkey;
