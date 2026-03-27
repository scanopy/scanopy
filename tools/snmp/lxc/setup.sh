#!/bin/bash
set -euo pipefail

# ══════════════════════════════════════════════════════════════════════
# SNMP Test Environment — Proxmox VM setup (self-contained)
#
# Paste this entire script into a Debian/Ubuntu VM terminal.
# Creates 6 snmpd instances on secondary IPs, each simulating a
# different network device with its own community string.
#
# Edit HOSTS/CIDR/IFACE below to match your network.
# ══════════════════════════════════════════════════════════════════════

HOSTS=(192.168.7.230 192.168.7.231 192.168.7.232 192.168.7.233 192.168.7.234 192.168.7.235)
CIDR="22"
IFACE="eth0"

COMMUNITIES=(netdefault netdefault secret42 secret42 public netdefault)
SYSNAMES=(switch-core-01 switch-access-01 router-gw-01 firewall-01 printer-lobby ap-wireless-01)

CONF_DIR="/etc/snmp-test"
DATA_DIR="$CONF_DIR/data"

echo "=== SNMP Test Environment Setup ==="

# ── 1. Install net-snmp ───────────────────────────────────────────────
if ! command -v snmpd &>/dev/null; then
    echo "Installing net-snmp..."
    apt-get update -qq && apt-get install -y -qq snmpd snmp gawk >/dev/null
fi
systemctl stop snmpd 2>/dev/null || true
systemctl disable snmpd 2>/dev/null || true
sleep 1

# ── 2. Add macvlan interfaces (each with unique MAC) ────────────────
echo "Configuring macvlan interfaces on $IFACE..."
for i in "${!HOSTS[@]}"; do
    ip="${HOSTS[$i]}"
    mvname="mv-snmp${i}"
    if ip link show "$mvname" &>/dev/null; then
        echo "  $mvname ($ip) already exists"
    else
        ip link add "$mvname" link "$IFACE" type macvlan mode bridge
        ip addr add "$ip/$CIDR" dev "$mvname"
        ip link set "$mvname" up
        mac=$(ip link show "$mvname" | awk '/ether/{print $2}')
        echo "  Created $mvname ($ip) mac=$mac"
    fi
done

# ── 3. Write pass handler ────────────────────────────────────────────
mkdir -p "$CONF_DIR" "$DATA_DIR"

cat > "$CONF_DIR/snmp-pass-handler.sh" << 'PASSEOF'
#!/bin/bash
DATA_FILE="$1"
REQUEST="$2"
OID="$3"

if [ ! -f "$DATA_FILE" ]; then
    echo "NONE"
    exit 0
fi

case "$REQUEST" in
    -g)
        LINE=$(awk -v oid="$OID" '$1 == oid { print; exit }' "$DATA_FILE")
        if [ -z "$LINE" ]; then
            echo "NONE"
            exit 0
        fi
        echo "$LINE" | awk '{ print $1; print $2; $1=""; $2=""; sub(/^  */, ""); print }'
        ;;
    -n)
        LINE=$(awk -v oid="$OID" '
            {
                if (oid_gt($1, oid)) {
                    print
                    exit
                }
            }
            function oid_gt(a, b,    na, nb, sa, sb, i) {
                na = split(a, sa, ".")
                nb = split(b, sb, ".")
                for (i = 1; i <= (na > nb ? na : nb); i++) {
                    ai = (i <= na) ? sa[i]+0 : -1
                    bi = (i <= nb) ? sb[i]+0 : -1
                    if (ai > bi) return 1
                    if (ai < bi) return 0
                }
                return 0
            }
        ' "$DATA_FILE")
        if [ -z "$LINE" ]; then
            echo "NONE"
            exit 0
        fi
        echo "$LINE" | awk '{ print $1; print $2; $1=""; $2=""; sub(/^  */, ""); print }'
        ;;
    *)
        echo "NONE"
        exit 0
        ;;
esac
PASSEOF
chmod +x "$CONF_DIR/snmp-pass-handler.sh"

# ── 4. Write MIB data files ──────────────────────────────────────────
echo "Writing MIB data..."

# switch-core-01 IF-MIB
cat > "$DATA_DIR/switch-core-01-iftable.txt" << 'EOF'
.1.3.6.1.2.1.2.2.1.1.1 integer 1
.1.3.6.1.2.1.2.2.1.1.2 integer 2
.1.3.6.1.2.1.2.2.1.1.3 integer 3
.1.3.6.1.2.1.2.2.1.1.4 integer 4
.1.3.6.1.2.1.2.2.1.2.1 string GigabitEthernet0/1
.1.3.6.1.2.1.2.2.1.2.2 string GigabitEthernet0/2
.1.3.6.1.2.1.2.2.1.2.3 string GigabitEthernet0/3
.1.3.6.1.2.1.2.2.1.2.4 string Vlan10
.1.3.6.1.2.1.2.2.1.3.1 integer 6
.1.3.6.1.2.1.2.2.1.3.2 integer 6
.1.3.6.1.2.1.2.2.1.3.3 integer 6
.1.3.6.1.2.1.2.2.1.3.4 integer 53
.1.3.6.1.2.1.2.2.1.5.1 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.2 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.3 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.4 gauge 0
.1.3.6.1.2.1.2.2.1.6.1 string 0:1a:2b:0:10:01
.1.3.6.1.2.1.2.2.1.6.2 string 0:1a:2b:0:10:02
.1.3.6.1.2.1.2.2.1.6.3 string 0:1a:2b:0:10:03
.1.3.6.1.2.1.2.2.1.6.4 string 0:1a:2b:0:10:00
.1.3.6.1.2.1.2.2.1.7.1 integer 1
.1.3.6.1.2.1.2.2.1.7.2 integer 1
.1.3.6.1.2.1.2.2.1.7.3 integer 1
.1.3.6.1.2.1.2.2.1.7.4 integer 1
.1.3.6.1.2.1.2.2.1.8.1 integer 1
.1.3.6.1.2.1.2.2.1.8.2 integer 1
.1.3.6.1.2.1.2.2.1.8.3 integer 1
.1.3.6.1.2.1.2.2.1.8.4 integer 1
.1.3.6.1.2.1.31.1.1.1.1.1 string Gi0/1
.1.3.6.1.2.1.31.1.1.1.1.2 string Gi0/2
.1.3.6.1.2.1.31.1.1.1.1.3 string Gi0/3
.1.3.6.1.2.1.31.1.1.1.1.4 string Vl10
.1.3.6.1.2.1.31.1.1.1.15.1 gauge 1000
.1.3.6.1.2.1.31.1.1.1.15.2 gauge 1000
.1.3.6.1.2.1.31.1.1.1.15.3 gauge 1000
.1.3.6.1.2.1.31.1.1.1.15.4 gauge 0
.1.3.6.1.2.1.31.1.1.1.18.1 string Uplink to switch-access-01
.1.3.6.1.2.1.31.1.1.1.18.2 string Uplink to router-gw-01
.1.3.6.1.2.1.31.1.1.1.18.3 string Server port
.1.3.6.1.2.1.31.1.1.1.18.4 string Management VLAN
EOF

# switch-core-01 LLDP
cat > "$DATA_DIR/switch-core-01-lldp.txt" << 'EOF'
.1.0.8802.1.1.2.1.3.1.0 integer 4
.1.0.8802.1.1.2.1.3.2.0 string 0:1a:2b:0:10:0
.1.0.8802.1.1.2.1.3.3.0 string switch-core-01
.1.0.8802.1.1.2.1.3.4.0 string Cisco IOS Software, C2960 Software (C2960-LANBASEK9-M), Version 15.2(7)E3
.1.0.8802.1.1.2.1.4.1.1.4.0.1.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.5.0.1.1 string 0:1a:2b:0:11:0
.1.0.8802.1.1.2.1.4.1.1.6.0.1.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.7.0.1.1 string Gi0/1
.1.0.8802.1.1.2.1.4.1.1.8.0.1.1 string GigabitEthernet0/1
.1.0.8802.1.1.2.1.4.1.1.9.0.1.1 string switch-access-01
.1.0.8802.1.1.2.1.4.1.1.10.0.1.1 string Cisco IOS Software, C3750 Software (C3750-IPSERVICESK9-M), Version 15.0(2)SE11
.1.0.8802.1.1.2.1.4.1.1.4.0.2.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.5.0.2.1 string 0:1a:2b:0:12:0
.1.0.8802.1.1.2.1.4.1.1.6.0.2.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.7.0.2.1 string ge-0/0/0
.1.0.8802.1.1.2.1.4.1.1.8.0.2.1 string ge-0/0/0
.1.0.8802.1.1.2.1.4.1.1.9.0.2.1 string router-gw-01
.1.0.8802.1.1.2.1.4.1.1.10.0.2.1 string Juniper Networks, Inc. JunOS 21.4R3-S5, MX204
EOF

# switch-access-01 IF-MIB
cat > "$DATA_DIR/switch-access-01-iftable.txt" << 'EOF'
.1.3.6.1.2.1.2.2.1.1.1 integer 1
.1.3.6.1.2.1.2.2.1.1.2 integer 2
.1.3.6.1.2.1.2.2.1.1.3 integer 3
.1.3.6.1.2.1.2.2.1.2.1 string GigabitEthernet0/1
.1.3.6.1.2.1.2.2.1.2.2 string GigabitEthernet0/2
.1.3.6.1.2.1.2.2.1.2.3 string GigabitEthernet0/3
.1.3.6.1.2.1.2.2.1.3.1 integer 6
.1.3.6.1.2.1.2.2.1.3.2 integer 6
.1.3.6.1.2.1.2.2.1.3.3 integer 6
.1.3.6.1.2.1.2.2.1.5.1 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.2 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.3 gauge 1000000000
.1.3.6.1.2.1.2.2.1.6.1 string 0:1a:2b:0:11:01
.1.3.6.1.2.1.2.2.1.6.2 string 0:1a:2b:0:11:02
.1.3.6.1.2.1.2.2.1.6.3 string 0:1a:2b:0:11:03
.1.3.6.1.2.1.2.2.1.7.1 integer 1
.1.3.6.1.2.1.2.2.1.7.2 integer 1
.1.3.6.1.2.1.2.2.1.7.3 integer 1
.1.3.6.1.2.1.2.2.1.8.1 integer 1
.1.3.6.1.2.1.2.2.1.8.2 integer 1
.1.3.6.1.2.1.2.2.1.8.3 integer 1
.1.3.6.1.2.1.31.1.1.1.1.1 string Gi0/1
.1.3.6.1.2.1.31.1.1.1.1.2 string Gi0/2
.1.3.6.1.2.1.31.1.1.1.1.3 string Gi0/3
.1.3.6.1.2.1.31.1.1.1.15.1 gauge 1000
.1.3.6.1.2.1.31.1.1.1.15.2 gauge 1000
.1.3.6.1.2.1.31.1.1.1.15.3 gauge 1000
.1.3.6.1.2.1.31.1.1.1.18.1 string Uplink to switch-core-01
.1.3.6.1.2.1.31.1.1.1.18.2 string Access port - Floor 2
.1.3.6.1.2.1.31.1.1.1.18.3 string Downlink to ap-wireless-01
EOF

# switch-access-01 LLDP
cat > "$DATA_DIR/switch-access-01-lldp.txt" << 'EOF'
.1.0.8802.1.1.2.1.3.1.0 integer 4
.1.0.8802.1.1.2.1.3.2.0 string 0:1a:2b:0:11:0
.1.0.8802.1.1.2.1.3.3.0 string switch-access-01
.1.0.8802.1.1.2.1.3.4.0 string Cisco IOS Software, C3750 Software (C3750-IPSERVICESK9-M), Version 15.0(2)SE11
.1.0.8802.1.1.2.1.4.1.1.4.0.1.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.5.0.1.1 string 0:1a:2b:0:10:0
.1.0.8802.1.1.2.1.4.1.1.6.0.1.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.7.0.1.1 string Gi0/1
.1.0.8802.1.1.2.1.4.1.1.8.0.1.1 string GigabitEthernet0/1
.1.0.8802.1.1.2.1.4.1.1.9.0.1.1 string switch-core-01
.1.0.8802.1.1.2.1.4.1.1.10.0.1.1 string Cisco IOS Software, C2960 Software (C2960-LANBASEK9-M), Version 15.2(7)E3
.1.0.8802.1.1.2.1.4.1.1.4.0.3.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.5.0.3.1 string 0:1a:2b:0:15:0
.1.0.8802.1.1.2.1.4.1.1.6.0.3.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.7.0.3.1 string eth0
.1.0.8802.1.1.2.1.4.1.1.8.0.3.1 string eth0
.1.0.8802.1.1.2.1.4.1.1.9.0.3.1 string ap-wireless-01
.1.0.8802.1.1.2.1.4.1.1.10.0.3.1 string Ubiquiti UniFi AP AC Pro, firmware 6.5.28
EOF

# router-gw-01 IF-MIB
cat > "$DATA_DIR/router-gw-01-iftable.txt" << 'EOF'
.1.3.6.1.2.1.2.2.1.1.1 integer 1
.1.3.6.1.2.1.2.2.1.1.2 integer 2
.1.3.6.1.2.1.2.2.1.1.3 integer 3
.1.3.6.1.2.1.2.2.1.2.1 string ge-0/0/0
.1.3.6.1.2.1.2.2.1.2.2 string ge-0/0/1
.1.3.6.1.2.1.2.2.1.2.3 string lo0.0
.1.3.6.1.2.1.2.2.1.3.1 integer 6
.1.3.6.1.2.1.2.2.1.3.2 integer 6
.1.3.6.1.2.1.2.2.1.3.3 integer 24
.1.3.6.1.2.1.2.2.1.5.1 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.2 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.3 gauge 0
.1.3.6.1.2.1.2.2.1.6.1 string 0:1a:2b:0:12:01
.1.3.6.1.2.1.2.2.1.6.2 string 0:1a:2b:0:12:02
.1.3.6.1.2.1.2.2.1.6.3 string
.1.3.6.1.2.1.2.2.1.7.1 integer 1
.1.3.6.1.2.1.2.2.1.7.2 integer 1
.1.3.6.1.2.1.2.2.1.7.3 integer 1
.1.3.6.1.2.1.2.2.1.8.1 integer 1
.1.3.6.1.2.1.2.2.1.8.2 integer 1
.1.3.6.1.2.1.2.2.1.8.3 integer 1
.1.3.6.1.2.1.31.1.1.1.1.1 string ge-0/0/0
.1.3.6.1.2.1.31.1.1.1.1.2 string ge-0/0/1
.1.3.6.1.2.1.31.1.1.1.1.3 string lo0.0
.1.3.6.1.2.1.31.1.1.1.15.1 gauge 1000
.1.3.6.1.2.1.31.1.1.1.15.2 gauge 1000
.1.3.6.1.2.1.31.1.1.1.15.3 gauge 0
.1.3.6.1.2.1.31.1.1.1.18.1 string Uplink to switch-core-01
.1.3.6.1.2.1.31.1.1.1.18.2 string Link to firewall-01
.1.3.6.1.2.1.31.1.1.1.18.3 string Loopback
EOF

# router-gw-01 LLDP
cat > "$DATA_DIR/router-gw-01-lldp.txt" << 'EOF'
.1.0.8802.1.1.2.1.3.1.0 integer 4
.1.0.8802.1.1.2.1.3.2.0 string 0:1a:2b:0:12:0
.1.0.8802.1.1.2.1.3.3.0 string router-gw-01
.1.0.8802.1.1.2.1.3.4.0 string Juniper Networks, Inc. JunOS 21.4R3-S5, MX204
.1.0.8802.1.1.2.1.4.1.1.4.0.1.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.5.0.1.1 string 0:1a:2b:0:10:0
.1.0.8802.1.1.2.1.4.1.1.6.0.1.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.7.0.1.1 string Gi0/2
.1.0.8802.1.1.2.1.4.1.1.8.0.1.1 string GigabitEthernet0/2
.1.0.8802.1.1.2.1.4.1.1.9.0.1.1 string switch-core-01
.1.0.8802.1.1.2.1.4.1.1.10.0.1.1 string Cisco IOS Software, C2960 Software (C2960-LANBASEK9-M), Version 15.2(7)E3
.1.0.8802.1.1.2.1.4.1.1.4.0.2.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.5.0.2.1 string 0:1a:2b:0:13:0
.1.0.8802.1.1.2.1.4.1.1.6.0.2.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.7.0.2.1 string port1
.1.0.8802.1.1.2.1.4.1.1.8.0.2.1 string port1
.1.0.8802.1.1.2.1.4.1.1.9.0.2.1 string firewall-01
.1.0.8802.1.1.2.1.4.1.1.10.0.2.1 string Fortinet FortiGate 60F v7.2.6 build1517 (GA.F)
EOF

# firewall-01 IF-MIB
cat > "$DATA_DIR/firewall-01-iftable.txt" << 'EOF'
.1.3.6.1.2.1.2.2.1.1.1 integer 1
.1.3.6.1.2.1.2.2.1.1.2 integer 2
.1.3.6.1.2.1.2.2.1.1.3 integer 3
.1.3.6.1.2.1.2.2.1.2.1 string port1
.1.3.6.1.2.1.2.2.1.2.2 string port2
.1.3.6.1.2.1.2.2.1.2.3 string port3
.1.3.6.1.2.1.2.2.1.3.1 integer 6
.1.3.6.1.2.1.2.2.1.3.2 integer 6
.1.3.6.1.2.1.2.2.1.3.3 integer 6
.1.3.6.1.2.1.2.2.1.5.1 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.2 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.3 gauge 1000000000
.1.3.6.1.2.1.2.2.1.6.1 string 0:1a:2b:0:13:01
.1.3.6.1.2.1.2.2.1.6.2 string 0:1a:2b:0:13:02
.1.3.6.1.2.1.2.2.1.6.3 string 0:1a:2b:0:13:03
.1.3.6.1.2.1.2.2.1.7.1 integer 1
.1.3.6.1.2.1.2.2.1.7.2 integer 1
.1.3.6.1.2.1.2.2.1.7.3 integer 1
.1.3.6.1.2.1.2.2.1.8.1 integer 1
.1.3.6.1.2.1.2.2.1.8.2 integer 1
.1.3.6.1.2.1.2.2.1.8.3 integer 1
.1.3.6.1.2.1.31.1.1.1.1.1 string port1
.1.3.6.1.2.1.31.1.1.1.1.2 string port2
.1.3.6.1.2.1.31.1.1.1.1.3 string port3
.1.3.6.1.2.1.31.1.1.1.15.1 gauge 1000
.1.3.6.1.2.1.31.1.1.1.15.2 gauge 1000
.1.3.6.1.2.1.31.1.1.1.15.3 gauge 1000
.1.3.6.1.2.1.31.1.1.1.18.1 string WAN - to router-gw-01
.1.3.6.1.2.1.31.1.1.1.18.2 string LAN - internal
.1.3.6.1.2.1.31.1.1.1.18.3 string DMZ
EOF

# firewall-01 LLDP
cat > "$DATA_DIR/firewall-01-lldp.txt" << 'EOF'
.1.0.8802.1.1.2.1.3.1.0 integer 4
.1.0.8802.1.1.2.1.3.2.0 string 0:1a:2b:0:13:0
.1.0.8802.1.1.2.1.3.3.0 string firewall-01
.1.0.8802.1.1.2.1.3.4.0 string Fortinet FortiGate 60F v7.2.6 build1517 (GA.F)
.1.0.8802.1.1.2.1.4.1.1.4.0.1.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.5.0.1.1 string 0:1a:2b:0:12:0
.1.0.8802.1.1.2.1.4.1.1.6.0.1.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.7.0.1.1 string ge-0/0/1
.1.0.8802.1.1.2.1.4.1.1.8.0.1.1 string ge-0/0/1
.1.0.8802.1.1.2.1.4.1.1.9.0.1.1 string router-gw-01
.1.0.8802.1.1.2.1.4.1.1.10.0.1.1 string Juniper Networks, Inc. JunOS 21.4R3-S5, MX204
EOF

# printer-lobby IF-MIB (no LLDP)
cat > "$DATA_DIR/printer-lobby-iftable.txt" << 'EOF'
.1.3.6.1.2.1.2.2.1.1.1 integer 1
.1.3.6.1.2.1.2.2.1.1.2 integer 2
.1.3.6.1.2.1.2.2.1.2.1 string Ethernet
.1.3.6.1.2.1.2.2.1.2.2 string USB
.1.3.6.1.2.1.2.2.1.3.1 integer 6
.1.3.6.1.2.1.2.2.1.3.2 integer 6
.1.3.6.1.2.1.2.2.1.5.1 gauge 100000000
.1.3.6.1.2.1.2.2.1.5.2 gauge 480000000
.1.3.6.1.2.1.2.2.1.6.1 string 0:1a:2b:0:14:01
.1.3.6.1.2.1.2.2.1.6.2 string 0:1a:2b:0:14:02
.1.3.6.1.2.1.2.2.1.7.1 integer 1
.1.3.6.1.2.1.2.2.1.7.2 integer 1
.1.3.6.1.2.1.2.2.1.8.1 integer 1
.1.3.6.1.2.1.2.2.1.8.2 integer 1
.1.3.6.1.2.1.31.1.1.1.1.1 string Ethernet
.1.3.6.1.2.1.31.1.1.1.1.2 string USB
.1.3.6.1.2.1.31.1.1.1.15.1 gauge 100
.1.3.6.1.2.1.31.1.1.1.15.2 gauge 480
.1.3.6.1.2.1.31.1.1.1.18.1 string Network port
.1.3.6.1.2.1.31.1.1.1.18.2 string USB port
EOF

# ap-wireless-01 IF-MIB
cat > "$DATA_DIR/ap-wireless-01-iftable.txt" << 'EOF'
.1.3.6.1.2.1.2.2.1.1.1 integer 1
.1.3.6.1.2.1.2.2.1.1.2 integer 2
.1.3.6.1.2.1.2.2.1.1.3 integer 3
.1.3.6.1.2.1.2.2.1.2.1 string eth0
.1.3.6.1.2.1.2.2.1.2.2 string ath0
.1.3.6.1.2.1.2.2.1.2.3 string ath1
.1.3.6.1.2.1.2.2.1.3.1 integer 6
.1.3.6.1.2.1.2.2.1.3.2 integer 71
.1.3.6.1.2.1.2.2.1.3.3 integer 71
.1.3.6.1.2.1.2.2.1.5.1 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.2 gauge 0
.1.3.6.1.2.1.2.2.1.5.3 gauge 0
.1.3.6.1.2.1.2.2.1.6.1 string 0:1a:2b:0:15:01
.1.3.6.1.2.1.2.2.1.6.2 string 0:1a:2b:0:15:02
.1.3.6.1.2.1.2.2.1.6.3 string 0:1a:2b:0:15:03
.1.3.6.1.2.1.2.2.1.7.1 integer 1
.1.3.6.1.2.1.2.2.1.7.2 integer 1
.1.3.6.1.2.1.2.2.1.7.3 integer 1
.1.3.6.1.2.1.2.2.1.8.1 integer 1
.1.3.6.1.2.1.2.2.1.8.2 integer 1
.1.3.6.1.2.1.2.2.1.8.3 integer 1
.1.3.6.1.2.1.31.1.1.1.1.1 string eth0
.1.3.6.1.2.1.31.1.1.1.1.2 string ath0
.1.3.6.1.2.1.31.1.1.1.1.3 string ath1
.1.3.6.1.2.1.31.1.1.1.15.1 gauge 1000
.1.3.6.1.2.1.31.1.1.1.15.2 gauge 867
.1.3.6.1.2.1.31.1.1.1.15.3 gauge 400
.1.3.6.1.2.1.31.1.1.1.18.1 string Uplink to switch-access-01
.1.3.6.1.2.1.31.1.1.1.18.2 string 5GHz radio
.1.3.6.1.2.1.31.1.1.1.18.3 string 2.4GHz radio
EOF

# ap-wireless-01 LLDP
cat > "$DATA_DIR/ap-wireless-01-lldp.txt" << 'EOF'
.1.0.8802.1.1.2.1.3.1.0 integer 4
.1.0.8802.1.1.2.1.3.2.0 string 0:1a:2b:0:15:0
.1.0.8802.1.1.2.1.3.3.0 string ap-wireless-01
.1.0.8802.1.1.2.1.3.4.0 string Ubiquiti UniFi AP AC Pro, firmware 6.5.28
.1.0.8802.1.1.2.1.4.1.1.4.0.1.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.5.0.1.1 string 0:1a:2b:0:11:0
.1.0.8802.1.1.2.1.4.1.1.6.0.1.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.7.0.1.1 string Gi0/3
.1.0.8802.1.1.2.1.4.1.1.8.0.1.1 string GigabitEthernet0/3
.1.0.8802.1.1.2.1.4.1.1.9.0.1.1 string switch-access-01
.1.0.8802.1.1.2.1.4.1.1.10.0.1.1 string Cisco IOS Software, C3750 Software (C3750-IPSERVICESK9-M), Version 15.0(2)SE11
EOF

# ── 5. Write snmpd configs ───────────────────────────────────────────
echo "Writing snmpd configs..."

D="$CONF_DIR/data"
H="$CONF_DIR/snmp-pass-handler.sh"

cat > "$CONF_DIR/snmpd-switch-core-01.conf" << EOF
agentAddress udp:${HOSTS[0]}:161
rocommunity netdefault
sysdescr Cisco IOS Software, C2960 Software (C2960-LANBASEK9-M), Version 15.2(7)E3
syscontact netops@example.com
sysname switch-core-01
syslocation Server Room A, Rack 1
sysobjectid .1.3.6.1.4.1.9.1.1208
sysservices 6
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/switch-core-01-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/switch-core-01-iftable.txt
pass .1.0.8802.1.1.2 /bin/bash $H $D/switch-core-01-lldp.txt
EOF

cat > "$CONF_DIR/snmpd-switch-access-01.conf" << EOF
agentAddress udp:${HOSTS[1]}:161
rocommunity netdefault
sysdescr Cisco IOS Software, C3750 Software (C3750-IPSERVICESK9-M), Version 15.0(2)SE11
syscontact netops@example.com
sysname switch-access-01
syslocation Floor 2, IDF B
sysobjectid .1.3.6.1.4.1.9.1.516
sysservices 6
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/switch-access-01-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/switch-access-01-iftable.txt
pass .1.0.8802.1.1.2 /bin/bash $H $D/switch-access-01-lldp.txt
EOF

cat > "$CONF_DIR/snmpd-router-gw-01.conf" << EOF
agentAddress udp:${HOSTS[2]}:161
rocommunity secret42
sysdescr Juniper Networks, Inc. JunOS 21.4R3-S5, MX204
syscontact netops@example.com
sysname router-gw-01
syslocation Server Room A, Rack 3
sysobjectid .1.3.6.1.4.1.2636.1.1.1.2.29
sysservices 76
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/router-gw-01-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/router-gw-01-iftable.txt
pass .1.0.8802.1.1.2 /bin/bash $H $D/router-gw-01-lldp.txt
EOF

cat > "$CONF_DIR/snmpd-firewall-01.conf" << EOF
agentAddress udp:${HOSTS[3]}:161
rocommunity secret42
sysdescr Fortinet FortiGate 60F v7.2.6 build1517 (GA.F)
syscontact netops@example.com
sysname firewall-01
syslocation Server Room A, Rack 2
sysobjectid .1.3.6.1.4.1.12356.101.1.1
sysservices 76
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/firewall-01-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/firewall-01-iftable.txt
pass .1.0.8802.1.1.2 /bin/bash $H $D/firewall-01-lldp.txt
EOF

cat > "$CONF_DIR/snmpd-printer-lobby.conf" << EOF
agentAddress udp:${HOSTS[4]}:161
rocommunity public
sysdescr HP LaserJet Pro MFP M428fdw, FW 2406334_042882
syscontact facilities@example.com
sysname printer-lobby
syslocation Lobby, Reception Desk
sysobjectid .1.3.6.1.4.1.11.2.3.9.1
sysservices 72
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/printer-lobby-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/printer-lobby-iftable.txt
EOF

cat > "$CONF_DIR/snmpd-ap-wireless-01.conf" << EOF
agentAddress udp:${HOSTS[5]}:161
rocommunity netdefault
sysdescr Ubiquiti UniFi AP AC Pro, firmware 6.5.28
syscontact netops@example.com
sysname ap-wireless-01
syslocation Floor 3, Ceiling
sysobjectid .1.3.6.1.4.1.41112.1.6.1
sysservices 6
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/ap-wireless-01-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/ap-wireless-01-iftable.txt
pass .1.0.8802.1.1.2 /bin/bash $H $D/ap-wireless-01-lldp.txt
EOF

# ── 6. Create systemd services ───────────────────────────────────────
echo "Creating systemd services..."
for i in "${!SYSNAMES[@]}"; do
    name="${SYSNAMES[$i]}"
    cat > "/etc/systemd/system/snmpd-${name}.service" << EOF
[Unit]
Description=SNMP Test Agent — ${name} (${HOSTS[$i]})
After=network.target

[Service]
Type=simple
ExecStart=/usr/sbin/snmpd -f -Lo -I -ifTable,-ifXTable -C -c ${CONF_DIR}/snmpd-${name}.conf
Restart=on-failure
RestartSec=2

[Install]
WantedBy=multi-user.target
EOF
done

# ── 7. Persist macvlan interfaces ────────────────────────────────────
if [ -d /etc/netplan ]; then
    echo "Persisting macvlan interfaces via netplan..."
    cat > /etc/netplan/60-snmp-test.yaml << EOF
network:
  version: 2
  ethernets:
$(for i in "${!HOSTS[@]}"; do
        mvname="mv-snmp${i}"
        mac=$(ip link show "$mvname" 2>/dev/null | awk '/ether/{print $2}')
        cat << INNER
    ${mvname}:
      match:
        macaddress: "${mac}"
      addresses:
        - ${HOSTS[$i]}/${CIDR}
INNER
done)
EOF
    netplan apply 2>/dev/null || true
elif [ -f /etc/network/interfaces ]; then
    echo "Persisting macvlan interfaces in /etc/network/interfaces..."
    for i in "${!HOSTS[@]}"; do
        mvname="mv-snmp${i}"
        if ! grep -q "$mvname" /etc/network/interfaces; then
            cat >> /etc/network/interfaces << EOF

auto ${mvname}
iface ${mvname} inet static
    address ${HOSTS[$i]}/${CIDR}
EOF
            fi
        fi
    done
fi

# ── 8. Start everything ──────────────────────────────────────────────
echo "Starting SNMP agents..."
systemctl daemon-reload
for name in "${SYSNAMES[@]}"; do
    systemctl enable "snmpd-${name}" --quiet
    systemctl restart "snmpd-${name}"
    printf "  %-28s started\n" "snmpd-${name}"
done

# ── 9. Verify ─────────────────────────────────────────────────────────
echo ""
echo "Verifying..."
sleep 1
all_ok=true
for i in "${!HOSTS[@]}"; do
    ip="${HOSTS[$i]}"
    community="${COMMUNITIES[$i]}"
    expected="${SYSNAMES[$i]}"

    result=$(snmpget -v2c -c "$community" -t 2 -r 1 "$ip" sysName.0 2>/dev/null | sed 's/.*= STRING: //' || echo "FAILED")
    if echo "$result" | grep -q "$expected"; then
        printf "  \033[0;32m✓\033[0m %-18s %-20s community=%-12s\n" "$ip" "$expected" "$community"
    else
        printf "  \033[0;31m✗\033[0m %-18s expected=%-20s got=%s\n" "$ip" "$expected" "$result"
        all_ok=false
    fi
done

echo ""
if $all_ok; then
    printf "\033[0;32mAll 6 SNMP test hosts are running.\033[0m\n"
else
    echo "Some hosts failed. Check: journalctl -u snmpd-<name>"
fi
