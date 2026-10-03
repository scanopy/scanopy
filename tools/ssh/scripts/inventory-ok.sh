#!/bin/sh
# Scanopy SSH credential script: host inventory for Debian/Ubuntu.
#
# Prints one JSON object on stdout whose keys fill Scanopy host fields. POSIX sh, and needs
# nothing beyond a base install (coreutils, sed, awk, iproute2). Fields it cannot read are left
# out rather than sent empty. /sys/class/dmi/id/product_serial is root-only on most systems, so
# an unprivileged login reports no serial unless SCANOPY_LAB_SERIAL supplies one.
#
# Lab overrides. The SSH lab runs every target on one VM, so each target's sshd sets these with
# SetEnv (see tools/ssh/setup.sh). On a real host they are unset and the script reports the host
# as it is:
#   SCANOPY_LAB_ADDR      report only the interface holding this address, and only this address
#   SCANOPY_LAB_HOSTNAME  hostname and sys_name
#   SCANOPY_LAB_SERIAL    serial_number
#   SCANOPY_LAB_LOCATION  sys_location
#   SCANOPY_LAB_CONTACT   sys_contact

set -u

# JSON forbids raw control characters, so they are dropped. Backslash and double quote are
# escaped. Input is expected to be ASCII or UTF-8.
json_escape() {
    printf '%s' "$1" | tr -d '\000-\037' | sed -e 's/\\/\\\\/g' -e 's/"/\\"/g'
}

# First line of a file, or nothing if it is unreadable or holds a firmware placeholder.
read_value() {
    [ -r "$1" ] || return 0
    v=$(head -n 1 "$1" 2>/dev/null | tr -d '\r\n')
    case "$v" in
        '' | 'To be filled by O.E.M.' | 'Default string' | 'Not Specified' | 'None' | 'System Product Name' | 'System manufacturer') return 0 ;;
    esac
    printf '%s' "$v"
}

out=""
sep=""
add_field() {
    [ -n "$2" ] || return 0
    out="${out}${sep}\"$1\":\"$(json_escape "$2")\""
    sep=","
}

dmi=/sys/class/dmi/id
hostname_value="${SCANOPY_LAB_HOSTNAME:-$(uname -n)}"
pretty_name=$(
    # shellcheck disable=SC1091
    [ -r /etc/os-release ] && . /etc/os-release
    printf '%s' "${PRETTY_NAME:-}"
)
kernel="$(uname -s) $(uname -r) $(uname -m)"
if [ -n "$pretty_name" ]; then
    sys_descr="$pretty_name, $kernel"
else
    sys_descr="$kernel"
fi

add_field hostname "$hostname_value"
add_field sys_name "$hostname_value"
add_field sys_descr "$sys_descr"
add_field sys_location "${SCANOPY_LAB_LOCATION:-}"
add_field sys_contact "${SCANOPY_LAB_CONTACT:-}"
add_field manufacturer "$(read_value "$dmi/sys_vendor")"
add_field model "$(read_value "$dmi/product_name")"
add_field serial_number "${SCANOPY_LAB_SERIAL:-$(read_value "$dmi/product_serial")}"
add_field firmware_revision "$(read_value "$dmi/bios_version")"
# The OS as an object (Scanopy's `os` key): family from the kernel's own name, the rest from
# /etc/os-release and uname. A kernel this script has no family for reports no `os`.
case "$(uname -s)" in
    Linux) os_family="Linux" ;;
    Darwin) os_family="MacOs" ;;
    FreeBSD) os_family="FreeBsd" ;;
    *) os_family="" ;;
esac
if [ -n "$os_family" ]; then
    os_release_field() {
        # shellcheck disable=SC1091
        [ -r /etc/os-release ] && . /etc/os-release
        eval "printf '%s' \"\${$1:-}\""
    }
    os="\"family\":\"$os_family\""
    for pair in name:NAME version:VERSION_ID codename:VERSION_CODENAME; do
        value=$(os_release_field "${pair#*:}")
        [ -n "$value" ] && os="${os},\"${pair%%:*}\":\"$(json_escape "$value")\""
    done
    os="${os},\"kernel_version\":\"$(json_escape "$(uname -r)")\""
    out="${out}${sep}\"os\":{${os}}"
    sep=","
fi

# ── Interfaces ────────────────────────────────────────────────────────
lab_addr="${SCANOPY_LAB_ADDR:-}"
primary_if=""
if [ -z "$lab_addr" ]; then
    primary_if=$(ip route show default 2>/dev/null |
        awk '{ for (i = 1; i < NF; i++) if ($i == "dev") { print $(i + 1); exit } }')
fi
chassis_mac=""
ifaces=""
isep=""
for path in /sys/class/net/*; do
    [ -d "$path" ] || continue
    name=${path##*/}
    [ "$name" = "lo" ] && continue

    cidrs=$(ip -o addr show dev "$name" scope global 2>/dev/null | awk '{ print $4 }')
    if [ -n "$lab_addr" ]; then
        cidrs=$(printf '%s\n' "$cidrs" | awk -v a="$lab_addr/" 'index($0, a) == 1')
        [ -n "$cidrs" ] || continue
        primary_if="$name"
    fi

    # Tunnel devices report 4- or 16-byte link addresses; only an Ethernet MAC is a MAC here.
    mac=$(read_value "$path/address")
    case "$mac" in
        00:00:00:00:00:00) mac="" ;;
        ??:??:??:??:??:??) ;;
        *) mac="" ;;
    esac
    # An interface with neither a MAC nor an address (sit0, gre0, ...) says nothing about the host.
    [ -z "$mac" ] && [ -z "$cidrs" ] && continue
    [ "$name" = "$primary_if" ] && chassis_mac="$mac"

    iface="\"name\":\"$(json_escape "$name")\""
    [ -n "$mac" ] && iface="${iface},\"mac\":\"$(json_escape "$mac")\""
    alias=$(read_value "$path/ifalias")
    [ -n "$alias" ] && iface="${iface},\"alias\":\"$(json_escape "$alias")\""
    # /sys reports Mb/s, and -1 or an unreadable file for a link with no speed.
    speed=$(read_value "$path/speed")
    case "$speed" in
        '' | *[!0-9]*) ;;
        0) ;;
        *) iface="${iface},\"speed_bps\":$((speed * 1000000))" ;;
    esac
    case "$(read_value "$path/operstate")" in
        up) iface="${iface},\"oper_status\":\"Up\"" ;;
        down) iface="${iface},\"oper_status\":\"Down\"" ;;
        dormant) iface="${iface},\"oper_status\":\"Dormant\"" ;;
        lowerlayerdown) iface="${iface},\"oper_status\":\"LowerLayerDown\"" ;;
    esac

    ifaces="${ifaces}${isep}{${iface}}"
    isep=","
done

add_field chassis_id "$chassis_mac"
out="${out}${sep}\"interfaces\":[${ifaces}]"

printf '{%s}\n' "$out"
