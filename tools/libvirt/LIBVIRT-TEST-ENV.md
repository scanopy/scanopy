# libvirt Test Environment

A KVM host with eight nested guests, for testing the libvirt credential end to end. The host is a
Debian 12 VM, `scanopy-libvirt-host`, on the Proxmox lab node (`pve`, 192.168.4.135), in pool
`scanopy-lab`. Its CPU type is `host`, so the guests run under nested KVM. Debian 12 ships
libvirt 9.0.

The host also runs Docker, so libvirt and a container runtime share one host as they do on many
real hypervisors.

## Host

| Setting | Value |
| --- | --- |
| Address | 192.168.7.172/22 on `br0`, which bridges the VM's NIC onto the lab LAN |
| `br-iso` | A bridge with no port and no address |
| libvirt network `default` | NAT, `virbr0`, 192.168.122.0/24 |
| User `scanopy` | In the `libvirt` group: full access, so guest-agent reads work |
| User `scanopy-ro` | Not in the group: only libvirt's read-only socket |
| Docker | `lab-web` (nginx on a bridge network) and `lab-mv` (macvlan on `br0` at 192.168.7.179) |

Both users accept `~/.ssh/libvirt-lab`, which `provision.sh` creates when it is missing.

## Guests

Every guest is Alpine 3.20 from its cloud image. Its NIC MACs are fixed (`52:54:00:7a:00:NN`,
libvirt's default prefix, which is locally administered), and its addresses are static except on
the NAT network.

| Domain | State | NICs (MAC suffix) | Guest agent | Exercises |
| --- | --- | --- | --- | --- |
| `kvm-noagent` | running | `br0` .173 (03) | channel, no agent | "agent not connected". Joined to its swept host by MAC |
| `kvm-nochannel` | running | `br0` .174 (04) | no channel | "agent not configured" |
| `kvm-agent` | running | `br0` .175 (05), `default` NAT (06) | running | Agent addresses, OS and hostname; two configured MACs |
| `kvm-paused` | paused | `br0` .176 (07) | none | State Paused |
| `kvm-stopped` | shut off | `br0`, .180 once started (08) | none | Recorded by MAC alone, state Stopped |
| `kvm-docker` | running | `br0` .177 (09) | running | `docker0` and a `br-*` network kept off its addresses; a macvlan link `mv-lab` at .178 as a network identity. The guest sets `arp_ignore=1`, `arp_announce=2` and loose `rp_filter`, so .178 answers ARP with `mv-lab`'s MAC rather than eth0's |
| `kvm-nat` | running | `default` NAT only (0a) | none | Warning: reachable only from the hypervisor |
| `kvm-isolated` | running | `br-iso`, 10.99.0.10 (0b) | none | Recorded by MAC with no warning: with no agent, and no address on `br-iso` for the host to learn it from, libvirt never sees its address |

## Setup

```sh
make libvirt-lab-provision
make libvirt-lab-status
```

`provision.sh` creates the VM from the Debian 12 cloud image on the node, bridges its NIC
(`setup.sh network`, which reboots it), then installs libvirt and Docker and defines the guests
(`setup.sh lab`). Run it again at any time: it reuses the VM and changes only what is missing.

It needs the Proxmox lab settings `tools/wol/` reads (`~/.config/scanopy-lab/proxmox.env`:
`PROXMOX_HOST`, `LAB_BRIDGE`) and root SSH to the node with `~/.ssh/proxmox-lab`. The VM takes
3.5 GB of the node's memory.

## Credentials for a scan

1. **SSH login** `libvirt-lab`: SSH Login (Key), username `scanopy`, the private key
   `~/.ssh/libvirt-lab`.
2. **SSH login** `libvirt-lab-ro`: the same, username `scanopy-ro`.
3. **libvirt** `KVM lab`: login `libvirt-lab`, connection URI `qemu:///system`. Assign it to the
   host at 192.168.7.172.
4. For the no-`virsh` case: an SSH login `ssh-lab` (SSH Login (Key), username `scanopy-ssh`, the
   key `~/.config/scanopy-lab/ssh/id_ed25519` from `tools/ssh/`), and a second libvirt credential
   `KVM lab (no virsh)` on it, assigned to the SSH lab's `inventory-ok` target, 192.168.7.160.
   `KVM lab` itself is refused there, since `scanopy` has no account on the SSH lab.

Scan from a daemon on the lab LAN (192.168.4.0/22), so the sweep learns the running guests' MACs
from ARP.

## Scenarios

Each is followed by a scan. `libvirt-test-env.sh scenario <name>` makes the change.

| Scenario | Change | Expected after the scan |
| --- | --- | --- |
| baseline | none | 192.168.7.172 has the libvirt (KVM) service and none of its own `docker0` addresses. Its Docker listens only on its socket, so no scan sees it. All eight domains sit under libvirt in the Workloads view with their states. `kvm-noagent`, `kvm-nochannel`, `kvm-agent` and `kvm-docker` are each one host, joined to the host the sweep found at their address. `kvm-agent` and `kvm-docker` show the OS and hostname their agent reads. The host at .178 sits under `kvm-docker` as a network identity. `lab-mv` (.179) is a host of its own. The run warns that `kvm-nat` is reachable only from the hypervisor. `kvm-isolated` is recorded by its MAC, with no warning. 192.168.7.160 shows `KVM lab (no virsh)` as not this service. |
| read-only | Change `KVM lab`'s login to `libvirt-lab-ro` | The same domains and states. No agent OS, hostname or identity is refreshed, and nothing fails. |
| start-stopped | `kvm-stopped` starts | `kvm-stopped` is Running, and its host gains 192.168.7.180 and stays one host. |
| undefine | `kvm-paused` is destroyed and undefined | Its host is deleted. |
| rename | `kvm-noagent` is renamed `kvm-renamed` | The same host, renamed. |
| reset | Every domain back to the baseline | |
