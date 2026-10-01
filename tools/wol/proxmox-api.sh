# shellcheck shell=bash disable=SC2034,SC2153  # colours are used by the sourcing scripts; PROXMOX_HOST comes from the env
# ══════════════════════════════════════════════════════════════════════
# Proxmox API and SSH helpers shared by provision.sh and wol-test-env.sh. Sourced, not run.
#
# Settings come from the environment, or from ~/.config/scanopy-lab/proxmox.env if it exists
# (outside the repo, because it holds the API token secret). See WOL-TEST-ENV.md, "One-time
# setup".
#
#   PROXMOX_HOST          node address for the API (port 8006) and SSH          required
#   PROXMOX_TOKEN_ID      API token id, user@realm!tokenname                    required
#   PROXMOX_TOKEN_SECRET  API token secret (a UUID)                             required
#   PROXMOX_NODE          node name (default: the first node the API lists)
#   PROXMOX_PORT          API port (default 8006)
#   PROXMOX_CACERT        CA bundle for the API certificate. Unset: the self-signed
#                         certificate is accepted without verification (curl -k).
#   PROXMOX_SSH_KEY       key for root@PROXMOX_HOST (default ~/.ssh/proxmox-lab)
#   PROXMOX_LAB_IP        the node's address on the lab bridge (default PROXMOX_HOST)
#   TEMPLATE_VMID         cloud-init template to clone                          provision only
#   LAB_BRIDGE            bridge the clones attach to, e.g. vmbr0               provision only
#   LAB_POOL              pool the clones go in (default scanopy-lab)
#   WOL_TARGET_NAME       name of the target VM (default scanopy-wol-target)
# ══════════════════════════════════════════════════════════════════════

PROXMOX_ENV_FILE="${PROXMOX_ENV_FILE:-$HOME/.config/scanopy-lab/proxmox.env}"
if [ -f "$PROXMOX_ENV_FILE" ]; then
    # shellcheck source=/dev/null
    . "$PROXMOX_ENV_FILE"
fi

PROXMOX_PORT="${PROXMOX_PORT:-8006}"
PROXMOX_SSH_KEY="${PROXMOX_SSH_KEY:-$HOME/.ssh/proxmox-lab}"
LAB_POOL="${LAB_POOL:-scanopy-lab}"
WOL_TARGET_NAME="${WOL_TARGET_NAME:-scanopy-wol-target}"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

die() {
    printf "${RED}✗${NC} %s\n" "$*" >&2
    exit 1
}

require_vars() {
    local v missing=()
    for v in "$@"; do
        [ -n "${!v:-}" ] || missing+=("$v")
    done
    if [ ${#missing[@]} -gt 0 ]; then
        die "unset: ${missing[*]} (export them or put them in $PROXMOX_ENV_FILE; see tools/wol/WOL-TEST-ENV.md)"
    fi
}

require_api() {
    require_vars PROXMOX_HOST PROXMOX_TOKEN_ID PROXMOX_TOKEN_SECRET
    command -v jq >/dev/null || die "jq is required (brew install jq)"
    if [ -z "${PROXMOX_NODE:-}" ]; then
        PROXMOX_NODE=$(pve GET /nodes | jq -r '.data[0].node')
        [ -n "$PROXMOX_NODE" ] && [ "$PROXMOX_NODE" != "null" ] || die "could not determine PROXMOX_NODE"
    fi
}

# pve METHOD PATH [name=value ...]: one API call, JSON on stdout. Form fields are URL-encoded.
pve() {
    local method="$1" path="$2"
    shift 2
    local tls=(-k)
    [ -n "${PROXMOX_CACERT:-}" ] && tls=(--cacert "$PROXMOX_CACERT")
    local data=() field
    for field in "$@"; do
        data+=(--data-urlencode "$field")
    done
    if [ "$method" = "GET" ] && [ ${#data[@]} -gt 0 ]; then
        data=(-G "${data[@]}")
    fi
    curl -sS --fail-with-body "${tls[@]}" -X "$method" \
        -H "Authorization: PVEAPIToken=${PROXMOX_TOKEN_ID}=${PROXMOX_TOKEN_SECRET}" \
        ${data[@]+"${data[@]}"} \
        "https://${PROXMOX_HOST}:${PROXMOX_PORT}/api2/json${path}"
}

# Wait for a task (UPID) to finish; fail if it did not end OK.
pve_wait_task() {
    local upid="$1" status exitstatus i
    for i in $(seq 1 300); do
        status=$(pve GET "/nodes/${PROXMOX_NODE}/tasks/${upid}/status")
        if [ "$(jq -r '.data.status' <<< "$status")" = "stopped" ]; then
            exitstatus=$(jq -r '.data.exitstatus' <<< "$status")
            [ "$exitstatus" = "OK" ] || die "task $upid ended: $exitstatus"
            return 0
        fi
        sleep 1
    done
    die "task $upid still running after ${i}s"
}

# VMID of the VM with this name in the lab pool, or nothing.
find_vmid() {
    local name="$1"
    pve GET /cluster/resources type=vm |
        jq -r --arg n "$name" --arg p "$LAB_POOL" \
            '[.data[] | select(.name == $n and .pool == $p) | .vmid][0] // empty'
}

# The MAC of a VM's net0, lowercase.
vm_mac() {
    pve GET "/nodes/${PROXMOX_NODE}/qemu/$1/config" |
        jq -r '.data.net0 // ""' |
        grep -oE '([0-9A-Fa-f]{2}:){5}[0-9A-Fa-f]{2}' | head -1 | tr 'A-F' 'a-f'
}

# status, qmpstatus and lock of a VM, space-separated.
vm_state() {
    pve GET "/nodes/${PROXMOX_NODE}/qemu/$1/status/current" |
        jq -r '[.data.status, (.data.qmpstatus // "-"), (.data.lock // "-")] | join(" ")'
}

node_ssh() {
    [ -f "$PROXMOX_SSH_KEY" ] || die "SSH key not found: $PROXMOX_SSH_KEY (set PROXMOX_SSH_KEY)"
    ssh -i "$PROXMOX_SSH_KEY" -o ConnectTimeout=10 "root@${PROXMOX_HOST}" "$@"
}

node_scp() {
    [ -f "$PROXMOX_SSH_KEY" ] || die "SSH key not found: $PROXMOX_SSH_KEY (set PROXMOX_SSH_KEY)"
    scp -i "$PROXMOX_SSH_KEY" -o ConnectTimeout=10 -q "$@"
}
