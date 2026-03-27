# SNMP Test Environment

6 simulated network devices running on a Proxmox VM, each with its own IP and community string on port 161.

| IP | Host | Community | Device |
|---|---|---|---|
| 192.168.7.230 | switch-core-01 | netdefault | Cisco C2960 |
| 192.168.7.231 | switch-access-01 | netdefault | Cisco C3750 |
| 192.168.7.232 | router-gw-01 | secret42 | Juniper MX204 |
| 192.168.7.233 | firewall-01 | secret42 | FortiGate 60F |
| 192.168.7.234 | printer-lobby | public | HP LaserJet M428 |
| 192.168.7.235 | ap-wireless-01 | netdefault | Ubiquiti UniFi AP |

## Setup

Paste the contents of `tools/snmp/lxc/setup.sh` into a root shell on a Debian/Ubuntu VM with primary IP 192.168.7.230/22.

Before pasting, verify:
- Interface is `eth0` (`ip link`) — edit `IFACE=` if different
- Primary IP is 192.168.7.230 — edit `HOSTS=()` if different

## Patch: migrate secondary IPs to macvlan (unique MACs)

If each device shares the host's MAC (secondary IPs on eth0), run on the VM:

```bash
IFACE=eth0; CIDR=22; HOSTS=(192.168.7.230 192.168.7.231 192.168.7.232 192.168.7.233 192.168.7.234 192.168.7.235); for i in "${!HOSTS[@]}"; do ip addr del "${HOSTS[$i]}/$CIDR" dev "$IFACE" 2>/dev/null; ip link del "mv-snmp${i}" 2>/dev/null; ip link add "mv-snmp${i}" link "$IFACE" type macvlan mode bridge; ip addr add "${HOSTS[$i]}/$CIDR" dev "mv-snmp${i}"; ip link set "mv-snmp${i}" up; done && sysctl -w net.ipv4.conf.all.arp_ignore=1 net.ipv4.conf.all.arp_announce=2 && for iface in mv-snmp0 mv-snmp1 mv-snmp2 mv-snmp3 mv-snmp4 mv-snmp5 eth0; do sysctl -w net.ipv4.conf.${iface}.arp_ignore=1 net.ipv4.conf.${iface}.arp_announce=2; done
```

Then flush the ARP cache on the scanning host (`sudo arp -a -d` on macOS).

## Patch: fix duplicate MIB registration

If snmpd logs show `duplicate registration: MIB modules ifTable and pass`, run:

```bash
for f in /etc/systemd/system/snmpd-*.service; do sed -i 's|snmpd -f -Lo -C|snmpd -f -Lo -I -ifTable,-ifXTable -C|' "$f"; done && systemctl daemon-reload && for name in switch-core-01 switch-access-01 router-gw-01 firewall-01 printer-lobby ap-wireless-01; do systemctl restart "snmpd-${name}"; done
```

## Verify

From your Mac:

```bash
make snmp-verify
```

Or manually:

```bash
snmpget -v2c -c secret42 -t 2 -r 1 192.168.7.232 sysName.0
```

## Manage services

```bash
# On the VM
systemctl status snmpd-router-gw-01
journalctl -u snmpd-router-gw-01 --no-pager -n 20
systemctl restart snmpd-router-gw-01
```
