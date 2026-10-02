# Wake-on-LAN Test Environment

A Proxmox VM that a Wake-on-LAN credential can wake. It tests the daemon's side of WoL: the magic
packet (102 bytes, or 108 with a SecureOn password), where the packet is addressed (the target
subnet's directed broadcast, or a configured `broadcast_address` relay), the three packets one
second apart, and the wait for the host to answer. It does not test NIC firmware, sleep states
on real hardware, or switches that drop broadcasts.

## How a VM is woken

Proxmox cannot wake a guest from a magic packet. `pvenode wakeonlan` sends a packet to wake a
Proxmox *node*, and a stopped guest has no running NIC to receive one.

`wol-listener.py` runs on the Proxmox node in place of the guest's NIC. It listens on UDP port 9,
logs every packet (source, size, target MAC, SecureOn password if present), finds the VM in pool
`scanopy-lab` whose `netN` MAC matches (from `qm config`), and:

| VM state | Listener runs | Stands in for |
|---|---|---|
| stopped | `qm start` | a host that is powered off (S5) |
| hibernated (`qm suspend --todisk`) | `qm start`, which resumes from the saved state | a hibernated host (S4) |
| paused in memory (`qm suspend`) | `qm resume` | not a sleep state; handled so a paused VM does not look broken |
| running | nothing, logs "already running" | an awake host |

Packets are handled one at a time and each reads the VM's state first, so the daemon's three
packets produce one `qm start` and two "already running" lines. There is no time-based repeat
suppression: a 30-second cooldown made a wake right after an earlier one look like a lost packet.

The listener is written from the AMD Magic Packet format (6 bytes of `0xFF`, then the target MAC
16 times, then an optional 6-byte SecureOn password), independently of Scanopy's sender. It also
accepts the magic sequence anywhere inside a larger payload, as real NICs do. It logs the
SecureOn password it received and does not enforce one: compare the logged value with the
credential's.

## Files

| File | Runs on | Does |
|---|---|---|
| `wol-listener.py` | Proxmox node, as root | Receives magic packets, starts or resumes the matching VM. Python 3 standard library only. `--dry-run` logs without starting anything; `--map-file` replaces pool lookup with a `<mac> <vmid>` file; `--port` repeats for several ports. |
| `wol-listener.service` | Proxmox node | systemd unit for the listener. Extra arguments go in `WOL_LISTENER_ARGS` in `/etc/default/wol-listener`. |
| `provision.sh` | Mac | Clones the template into the pool as `scanopy-wol-target`, attaches it to the lab bridge, starts it once, and installs the listener on the node. |
| `wol-test-env.sh` | Mac | `sleep [--hibernate]`, `status`, `verify <address> [port]`, `listener-log [-f]`. |
| `proxmox-api.sh` | Mac (sourced) | Proxmox API (`curl` with a `PVEAPIToken` header) and SSH helpers. |

## Two test paths

### Path 1: Mac daemon through the relay

The daemon under test runs on the Mac (`en0`, `192.168.4.0/22`). Set the credential's
`broadcast_address` to the node's address on the lab bridge (`PROXMOX_LAB_IP`, 192.168.4.135). The daemon then sends the packet as unicast UDP to the node, the
listener receives it directly, and the VM starts.

This covers the relay addressing, the packet format and the wait loop against a real boot delay.

### Path 2: directed broadcast, no override

The credential has no `broadcast_address`. Verified 2026-10-01: the Mac and the node share an L2
segment (both on `192.168.4.0/22`), and a packet the Mac sent to `192.168.7.255` reached the
listener and started the VM, so the Mac daemon covers this path too. A lab daemon VM is only needed
if that changes. The daemon sends to the target subnet's directed broadcast, which goes out
as an Ethernet broadcast on the bridge, and the listener receives it on the node's bridge
address. This needs the node to hold an address in the target's subnet on `LAB_BRIDGE`.

If it does, `tools/wol/provision.sh daemon-vm` clones `scanopy-wol-daemon` from the same template. Install
a Scanopy daemon in it the usual way and point it at the same server.

This covers the directed-broadcast address computation and the default path with no override.

## One-time setup

These need Maya once. Everything after this is scripted.

1. **SSH key on the node.** The listener is installed and its log read over SSH as root.

   ```sh
   ssh-keygen -t ed25519 -f ~/.ssh/proxmox-lab -N ''
   ssh-copy-id -i ~/.ssh/proxmox-lab.pub root@<PROXMOX_HOST>
   ```

2. **Pool, user and API token.** On the node:

   ```sh
   pveum pool add scanopy-lab --comment "Scanopy WoL lab"
   pveum user add scanopy-lab@pve
   pveum user token add scanopy-lab@pve wol --privsep 1      # prints the secret once
   pveum acl modify /pool/scanopy-lab --users scanopy-lab@pve --roles PVEVMAdmin
   pveum acl modify /pool/scanopy-lab --tokens 'scanopy-lab@pve!wol' --roles PVEVMAdmin
   ```

   A privilege-separated token gets the intersection of its own and its user's permissions, so
   every ACL is granted to both. `PVEVMAdmin` on the pool is not enough to clone on its own:

   - Put the template in the pool, so the token has `VM.Clone` on it:
     `pveum pool modify scanopy-lab --vms <TEMPLATE_VMID>`
   - Grant `PVEDatastoreUser` on the storage the clone's disks and the hibernation state land
     on: `pveum acl modify /storage/<storage> --users scanopy-lab@pve --tokens 'scanopy-lab@pve!wol' --roles PVEDatastoreUser`
   - On Proxmox VE 8 and later, grant use of the bridge:
     `pveum acl modify /sdn/zones/localnetwork/<LAB_BRIDGE> --users scanopy-lab@pve --tokens 'scanopy-lab@pve!wol' --roles PVESDNUser`

3. **Template.** `provision.sh` clones a cloud-init template VM to make the WoL target. It needs
   one network device (`net0`, the MAC the listener matches), ACPI on (the default, so `sleep`
   shuts it down cleanly), and a cloud-init drive (so the clone gets its address). Its id is
   `TEMPLATE_VMID`. An existing Debian or Ubuntu cloud-init template with one NIC works as is.

   To make one, first find two names on the node:

   - **VM storage** (`<storage>` below): where VM disks live. The same storage gets
     `PVEDatastoreUser` in step 2. Run `pvesm status --content images`
     and pick an `active` one; `local-lvm` on a default install, `local-zfs` on a ZFS install. In
     the UI: Datacenter > Storage, any entry whose Content includes "Disk image".
   - **Lab bridge** (`<bridge>` below, also `LAB_BRIDGE` in step 4): the bridge the SNMP lab VM
     sits on, so the target shares its segment. Run `qm list` to get that VM's id, then
     `qm config <id> | grep ^net` and read `bridge=` (e.g. `vmbr0`). In the UI: the VM >
     Hardware > Network Device. `ip -br addr show type bridge` lists every bridge with the node's
     own address on it; the node's address on the lab bridge is `PROXMOX_LAB_IP` in step 4. If the
     node has none there, the relay path (path 1) has nothing to send to; add one before testing it.

   Then, as root on the node (Debian 12, id 9000):

   ```sh
   cd /var/lib/vz/template/iso
   wget https://cloud.debian.org/images/cloud/bookworm/latest/debian-12-genericcloud-amd64.qcow2

   qm create 9000 --name debian12-cloud --memory 1024 --cores 1 \
     --net0 virtio,bridge=<bridge> --scsihw virtio-scsi-pci
   qm importdisk 9000 debian-12-genericcloud-amd64.qcow2 <storage>
   qm set 9000 --scsi0 <storage>:vm-9000-disk-0 --boot order=scsi0
   qm set 9000 --ide2 <storage>:cloudinit --serial0 socket --vga serial0
   qm set 9000 --ciuser debian --sshkeys ~/.ssh/authorized_keys
   qm template 9000
   ```

   `qm importdisk` prints the disk's real name on its last line; on directory storage (`local`)
   it is `<storage>:9000/vm-9000-disk-0.raw`, so use that in the `--scsi0` line. If 9000 is
   taken, `pvesh get /cluster/nextid` gives a free id.

4. **Settings file.** `~/.config/scanopy-lab/proxmox.env`, outside the repo since it holds the
   token secret, `chmod 600`:

   ```sh
   PROXMOX_HOST=<node address>
   PROXMOX_NODE=<node name>                  # optional; defaults to the first node
   PROXMOX_TOKEN_ID='scanopy-lab@pve!wol'
   PROXMOX_TOKEN_SECRET=<uuid from step 2>
   PROXMOX_LAB_IP=<node address on LAB_BRIDGE>   # defaults to PROXMOX_HOST
   TEMPLATE_VMID=<template id>
   LAB_BRIDGE=<bridge, e.g. vmbr0>
   WOL_TARGET_IPCONFIG='ip=<free lab address>/22,gw=<gateway>'   # default ip=dhcp
   WOL_TARGET_IP=<that address>              # verify then also waits for ping
   # PROXMOX_CACERT=<path>                   # unset: the self-signed API cert is not verified
   ```

   `192.168.7.170` and `.171` sit just above the SSH lab block and below the SNMP block. Check
   they are free on the network before using them for the target and the daemon VM.

5. **Firewall.** If the Proxmox firewall is on at node level, allow UDP 9 in on the lab bridge.

## Running a test

```sh
make wol-lab-provision        # clone, attach to LAB_BRIDGE, start once, install the listener
```

Scan the lab network once while the target is up, so Scanopy records its MAC and address. Then
check the listener and the addressing with this Mac's own sender, before involving the daemon:

```sh
make wol-lab-sleep                            # or wol-lab-hibernate
make wol-lab-verify ADDR=<PROXMOX_LAB_IP>     # sends 3 packets, waits for the VM (and ping)
```

`verify` refuses to run while the target is awake. `WOL_SECUREON=<12 hex digits>` appends a
SecureOn password; `PORT=7` sends to another port (the listener needs `--port 7` for that).

Then the real test:

1. `make wol-lab-sleep` (or `make wol-lab-hibernate`).
2. Create the WoL credential for the target host (template below). For path 1 set
   `broadcast_address` to `PROXMOX_LAB_IP`; for path 2 leave it out.
3. Trigger the wake from Scanopy.
4. Watch the listener: `tools/wol/wol-test-env.sh listener-log -f`.

Expected:

- Three log lines one second apart from the daemon's address (path 2) or the Mac's (path 1),
  `form=standard` and 102 bytes, or `form=secureon` and 108 bytes with the configured password.
  `mac=` is the target's MAC and `vm=` its VMID.
- The first line is followed by `qm start`; the next two by `already running, nothing to do`.
- The daemon reports the host awake within `wait_seconds`.

Failure cases worth running:

| Setup | Expected |
|---|---|
| `wait_seconds` shorter than the VM's boot | The daemon reports that the host did not answer. The VM still comes up. |
| Listener in dry run (`WOL_LISTENER_ARGS=--dry-run` in `/etc/default/wol-listener`, then `make wol-lab-listener`) | Packets logged, VM stays down, the daemon's wait runs out. Exercises the timeout path with a packet that did arrive. |
| Path 1 credential with no `broadcast_address` | No log line on the node. The daemon reports that the host did not answer. |
| Wrong `broadcast_address` | No log line on the node. |

## WoL credential template

`POST /api/v1/credentials` with `Authorization: Bearer <api key>`. The `credential_type` fields
are the `WakeOnLan` variant of `components["schemas"]["CredentialType"]` in
`ui/src/lib/api/schema.d.ts`.

```sh
jq -n --arg org "<organization id>" --arg host "<target host id>" --arg relay "<PROXMOX_LAB_IP>" '{
  name: "wol-lab relay",
  organization_id: $org,
  credential_type: {
    type: "WakeOnLan",
    broadcast_address: $relay,
    port: 9,
    secure_on_password: null,
    wait_seconds: 120
  },
  assigned_network_ids: [],
  host_assignments: [{host_id: $host, ip_address_ids: null}],
  tags: []
}' |
curl -sS -X POST "<server url>/api/v1/credentials" \
  -H "Authorization: Bearer <api key>" \
  -H "Content-Type: application/json" \
  --data-binary @-
```

For path 2, drop `broadcast_address`. A SecureOn password is a secret like the others,
`{"mode": "Inline", "value": "aa:bb:cc:dd:ee:ff"}`: six bytes written as a MAC address.

## Status and logs

```sh
make wol-lab-status      # target VMID, MAC, status/qmpstatus/lock, listener health
make wol-lab-log         # last 50 listener lines
tools/wol/wol-test-env.sh listener-log -f
```
