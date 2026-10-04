-- no-transaction
--
-- Index `hosts.virtualization_interface_id`, so deleting an interface finds the hosts it presents
-- (`ON DELETE SET NULL`) without scanning `hosts`. Outside a transaction because the index is
-- built `CONCURRENTLY`.

SET lock_timeout = '5s';
SET statement_timeout = '0';

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_hosts_virtualization_interface_id
    ON hosts (virtualization_interface_id)
    WHERE virtualization_interface_id IS NOT NULL;
