#!/bin/bash
set -euo pipefail

# SNMP Test Environment — manages 6 snmpd instances on a Proxmox LXC
# Subnet: 192.168.4.0/22 (hosts at 192.168.7.230–235)
# Usage: tools/snmp/snmp-test-env.sh verify|status|ssh-setup

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SNMPGET="${SNMPGET:-/opt/homebrew/opt/net-snmp/bin/snmpget}"

HOSTS=(192.168.7.230 192.168.7.231 192.168.7.232 192.168.7.233 192.168.7.234 192.168.7.235)
COMMUNITIES=(netdefault netdefault secret42 secret42 public netdefault)
SYSNAMES=("switch-core-01" "switch-access-01" "router-gw-01" "firewall-01" "printer-lobby" "ap-wireless-01")

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

cmd_verify() {
    echo "Verifying SNMP test hosts..."
    echo ""
    local all_ok=true
    for i in "${!HOSTS[@]}"; do
        local host="${HOSTS[$i]}"
        local community="${COMMUNITIES[$i]}"
        local expected="${SYSNAMES[$i]}"

        local result
        result=$("$SNMPGET" -v2c -c "$community" -t 2 -r 1 "$host" sysName.0 2>/dev/null | sed 's/.*= STRING: //' || echo "FAILED")
        if echo "$result" | grep -q "$expected"; then
            printf "  ${GREEN}✓${NC} %-18s  %-20s  community=%-12s\n" "$host" "$expected" "$community"
        else
            printf "  ${RED}✗${NC} %-18s  expected=%-20s  got=%s\n" "$host" "$expected" "$result"
            all_ok=false
        fi
    done

    echo ""
    if $all_ok; then
        printf "${GREEN}All 6 SNMP test hosts are reachable.${NC}\n"
        echo ""
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo "  LXC hosts on 192.168.4.0/22"
        echo ""
        printf "  %-18s %-22s %s\n" "IP" "Host" "Community"
        printf "  %-18s %-22s %s\n" "────────────────" "────────────────────" "────────────"
        for i in "${!HOSTS[@]}"; do
            printf "  %-18s %-22s %s\n" "${HOSTS[$i]}" "${SYSNAMES[$i]}" "${COMMUNITIES[$i]}"
        done
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    else
        printf "${YELLOW}Some hosts are unreachable. Is the LXC running?${NC}\n"
        echo "  Check with: ssh root@${HOSTS[0]} 'systemctl list-units snmpd-*'"
    fi
}

cmd_status() {
    echo "SNMP Test Environment Status"
    echo "=============================="
    echo ""
    echo "Checking reachability (ICMP)..."
    for i in "${!HOSTS[@]}"; do
        local host="${HOSTS[$i]}"
        local name="${SYSNAMES[$i]}"
        if ping -c 1 -W 1 "$host" &>/dev/null; then
            printf "  ${GREEN}✓${NC} %-18s  %s\n" "$host" "$name"
        else
            printf "  ${RED}✗${NC} %-18s  %s  (unreachable)\n" "$host" "$name"
        fi
    done
}

case "${1:-}" in
    verify)
        cmd_verify
        ;;
    status)
        cmd_status
        ;;
    *)
        echo "Usage: $0 {verify|status}"
        echo ""
        echo "  verify — Query each SNMP host and check sysName"
        echo "  status — Ping each host to check reachability"
        echo ""
        echo "LXC setup: copy tools/snmp/ to the container and run lxc/setup.sh"
        exit 1
        ;;
esac
