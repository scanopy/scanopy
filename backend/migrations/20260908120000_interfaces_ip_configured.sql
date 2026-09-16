-- Whether the device's own SNMP ipAddrTable binds an IP address to this interface (GH #668).
--
-- A Windows host can expose one MAC address on its real NIC and on several NDIS filter/LWF
-- pseudo-interfaces layered on top of it (WFP Native MAC Layer, QoS Packet Scheduler, WFP 802.3
-- filters) -- all reporting an ethernet ifType, so none is excluded from MAC-based LLDP port
-- resolution by if_type alone, and the lookup gives up as ambiguous. ipAddrTable only ever binds
-- an address to a real IP-stack adapter -- a filter driver is never a separate one -- so it
-- distinguishes the physical interface among candidates that otherwise look identical.
--
-- Independent of ip_address_id: that FK is set server-side by plan_interface_ip_links, which
-- requires the MAC to be carried by exactly one interface on the host before it links anything,
-- so it stays NULL on precisely the hosts this column exists to help. This column needs no such
-- guard -- it is the daemon's own ipAddrTable-to-ifIndex read, independent of whether the MAC is
-- shared.
--
-- Additive and prod-safe: ADD COLUMN with a constant default is metadata-only (no rewrite), which
-- matters because `interfaces` carries SCD2 history rows. `false` is the correct default for every
-- existing row: none of them were populated with this signal, and reading `false` is exactly as
-- conservative as today's behaviour (the MAC-uniqueness tie-break simply cannot use them yet,
-- which is no worse than the ambiguity they already resolved to). No backfill: the value only
-- becomes meaningful once a daemon that reports it runs a fresh scan.
--
-- No contract step: nothing is renamed or dropped, and older servers/daemons ignore the column.

SET lock_timeout = '5s';
SET statement_timeout = '30s';

ALTER TABLE interfaces
    ADD COLUMN IF NOT EXISTS ip_configured BOOLEAN NOT NULL DEFAULT false;
