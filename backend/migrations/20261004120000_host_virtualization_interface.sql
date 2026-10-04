-- The interface on the virtualizing host that presents a host.
--
-- A network identity (an address and MAC a guest presents from one of its own interfaces, such as
-- a macvlan link inside a VM) is linked to its guest by `virtualization_service_id`. This names
-- which of the guest's interfaces presents it. A real column with a foreign key, for the same
-- reason `virtualization_service_id` is one (20260803120001): a reference that no longer resolves
-- fails the write instead of surviving as a value nothing matches, and `ON DELETE SET NULL`
-- clears it when the interface goes away.
--
-- No backfill: no host holds a value yet. The JSONB field it replaces
-- (`virtualization_metadata->'details'->>'interface'`, an interface name) was never released.
--
-- Additive and prod-safe: a nullable `ADD COLUMN` is metadata-only, and the foreign key is added
-- `NOT VALID` here and validated in the next migration. Older servers ignore the column.

SET lock_timeout = '5s';
SET statement_timeout = '30s';

ALTER TABLE hosts ADD COLUMN virtualization_interface_id UUID;

ALTER TABLE hosts
    ADD CONSTRAINT hosts_virtualization_interface_id_fkey
    FOREIGN KEY (virtualization_interface_id) REFERENCES interfaces(id) ON DELETE SET NULL
    NOT VALID;
