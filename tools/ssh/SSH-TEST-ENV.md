# SSH Test Environment

Eight SSH targets for testing the SshPassword and SshKey credentials end to end. The daemon logs
in, runs the credential's script on the target, and applies the JSON object the script prints to
the host's fields. Each target is a dedicated `sshd` on its own address, `192.168.7.160` to
`.167`, all running on the SNMP lab VM (`root@192.168.4.21`, see `tools/snmp/SNMP-TEST-ENV.md`).

Each case is set by the script on the credential. The targets differ in their identity (hostname,
serial, address) and, for `password`, in which login method they accept. Each case has its own
address so each shows up as its own host in Scanopy.

## Targets

| IP | Case | Auth accepted | Script on the credential |
|---|---|---|---|
| 192.168.7.160 | inventory-ok | key | `scripts/inventory-ok.sh` |
| 192.168.7.161 | password | password | `scripts/inventory-ok.sh` |
| 192.168.7.162 | exit-nonzero | key | `scripts/exit-nonzero.sh` |
| 192.168.7.163 | bad-json | key | `scripts/bad-json.sh` |
| 192.168.7.164 | timeout | key | `scripts/timeout.sh` |
| 192.168.7.165 | oversized | key | `scripts/oversized.sh` |
| 192.168.7.166 | unknown-keys | key | `scripts/unknown-keys.sh` |
| 192.168.7.167 | hostkey-rotate | key | `scripts/inventory-ok.sh` |

Every target logs in user `scanopy-ssh`. The key-only targets refuse passwords, so an SshPassword
credential assigned to one of them tests an authentication failure. The `password` target
refuses keys, so only an SshPassword credential gets in there.

The list lives in `targets.env`, which both `setup.sh` and `ssh-test-env.sh` read. It is append
only: Scanopy pins host keys per IP:port, so renumbering a target turns into a changed-key
refusal on every address that moved.

## Expected scan outcome per case

| Case | Expected outcome |
|---|---|
| inventory-ok | Host named `ssh-inventory-ok`. `sys_descr` is the VM's OS and kernel (`Ubuntu ..., Linux ... x86_64`). `manufacturer`, `model` and `firmware_revision` come from the VM's DMI data (QEMU values). `serial_number` is `SSHLAB-007160`. `sys_location` and `sys_contact` are set. One interface, `mv-ssh0`, with its own MAC and `192.168.7.160/22`; `chassis_id` is that MAC. |
| password | Same as inventory-ok, as `ssh-password` with serial `SSHLAB-007161`. An SshKey credential fails authentication here. |
| exit-nonzero | The run fails with exit status 3 and the stderr line `scanopy-ssh-lab: simulated failure, exiting 3`. Nothing is applied. The script prints valid JSON first, so a host renamed `applied-despite-exit-3` means the exit status was ignored. |
| bad-json | Exit 0, output fails to parse. Reported as a parse failure, nothing applied. A host renamed `applied-from-bad-json` means a partial parse was applied. |
| timeout | The script sleeps 120 s. With the default `timeout_seconds` of 60 the run is reported as a timeout after 60 s, nothing is applied, and the rest of the scan completes. Raising the timeout above 120 s makes it succeed as `applied-after-timeout`. |
| oversized | The script prints about 80 KiB of valid JSON. The daemon stops at its 64 KiB cap, reports the output as oversized, and applies nothing. A host renamed `applied-from-oversized` means the cap was not enforced. |
| unknown-keys | `hostname` (`ssh-unknown-keys`), `sys_descr`, `model` and the interface `lab0` with `192.168.7.166/22` are applied. `favorite_color`, `rack_units` and `extra_info` are reported as unknown keys and not applied. |
| hostkey-rotate | The first scan succeeds like inventory-ok (as `ssh-hostkey-rotate`) and pins the host key. After `make ssh-lab-rotate-hostkey IP=192.168.7.167`, the next scan refuses the changed key, applies nothing, and keeps the original pin. |

## How the lab is built

### Addresses and links

`ssh-lab-network.service` is a oneshot unit that runs `/usr/local/bin/ssh-lab-network-up.sh`
(installed from `lab-network-up.sh`) at boot and on every deploy. Target `i` gets a macvlan link
`mv-ssh<i>` on `eth0` holding `SSH_HOSTS[i]/22` and nothing else. The address block
`192.168.7.160` to `.169` sits below the SNMP block, which starts at `.192`.

The SSH and SNMP labs share the VM without touching each other's network state:

- Each lab owns its own link names. The SNMP reconciler deletes only `mv-snmp*` links it has no
  device for, and this one only `mv-ssh*`. A container run of `lab-network-up.sh` next to an
  `mv-snmp0` link left that link alone and was idempotent across repeated runs.
- Neither lab writes a netplan file for its links. `tools/snmp/lxc/setup.sh` deletes
  `/etc/netplan/60-snmp-test.yaml` and runs `netplan apply` on every deploy, and netplan cannot
  create a macvlan anyway.
- Each lab has its own env file (`/etc/ssh-test/lab-network.env`) and its own sysctl file
  (`/etc/sysctl.d/60-ssh-lab-arp.conf`). Both labs set the same ARP values, so either file alone
  keeps every macvlan answering ARP with its own MAC.

### Port 22 and the VM's own sshd

The VM's own sshd listens on `0.0.0.0:22`, and Linux refuses to bind `192.168.7.160:22` while a
wildcard listener holds the port (`Bind to port 22 on 192.168.7.160 failed: Address already in
use`, reproduced in a container). Changing the main sshd's listen address is off limits, so each
target sshd listens on `<ip>:2222` instead, and an nftables table of the lab's own,
`ip scanopy_ssh_lab`, rewrites `<ip>:22` to `<ip>:2222` on arrival. To Scanopy every target is
an sshd on port 22. The rule matches only the lab addresses, so `192.168.4.21:22` still reaches
the main sshd. A port scan of a target also finds 2222 open.

### One sshd per target

Each target has a config and an ed25519 host key under `/etc/ssh-test/targets/<ip>/`, and runs
as `sshd-test@<ip>.service`. The config includes nothing from `/etc/ssh/`. `setup.sh` checks each
config with `sshd -t -f` before replacing the previous one, and the unit runs the same check
before every start. `/etc/ssh/`, `ssh.service` and `ssh.socket` are never written, restarted or
reloaded.

Host keys are generated once and kept across deploys, since a deploy that regenerated them would
turn every target into a changed-key refusal. `rotate-hostkey` is the only thing that replaces
one.

### Target identity

All eight targets run on one VM. Without help, `inventory-ok.sh` would report the same hostname,
serial and full interface list (the VM's management address and every lab address) from every
target, and Scanopy would merge them into one host. Each target's sshd sets `SCANOPY_LAB_ADDR`,
`SCANOPY_LAB_HOSTNAME`, `SCANOPY_LAB_SERIAL`, `SCANOPY_LAB_LOCATION` and `SCANOPY_LAB_CONTACT`
with `SetEnv`. `inventory-ok.sh` reports only the interface holding `SCANOPY_LAB_ADDR` and takes
its name and serial from the others. On a host without those variables it reports the host as it
is. `make ssh-lab-verify` fails any target whose output names another address.

### The credential scripts

`scripts/*.sh` are the bodies to paste into a credential's `script` field. All are POSIX `sh`
and need nothing beyond a base Debian or Ubuntu install (coreutils, sed, awk, iproute2).
`inventory-ok.sh` reads `/etc/os-release`, `uname`, `/sys/class/dmi/id/*` where readable and
`/sys/class/net` plus `ip -o addr` for interfaces, and escapes every string it emits. It omits a
field it cannot read rather than sending it empty. `product_serial` is root-only on most systems,
so on a real host an unprivileged login reports no serial.

## Setup

Needs the SNMP VM key (`~/.ssh/snmp-test-vm`, or `SNMP_SSH_KEY` / `SSH_LAB_VM_KEY`) and `jq` on
the Mac.

```sh
make ssh-lab-deploy
```

This clears `/root/ssh-test` on the VM (`scp -r` into an existing directory nests the tree one
level deeper and leaves `setup.sh` running the stale copy), copies `tools/ssh` there, runs
`setup.sh`, then runs `make ssh-lab-verify`. The first run creates the lab's client key and
password in `~/.config/scanopy-lab/ssh/` (`id_ed25519`, `password`), outside the repo. Later
deploys reuse them, so credentials created in Scanopy keep working. `setup.sh` prints the
password and each target's host key fingerprint.

Rerun the same command after changing anything under `tools/ssh/`. It is idempotent: it keeps
host keys, MACs (links are reconciled, not rebuilt) and the password.

## Verify

```sh
make ssh-lab-verify
```

From the Mac, runs each target's script over plain `ssh` as `scanopy-ssh`, the way the daemon
will (`ssh ... 'sh -s' < script`), and checks the result for that case:

| Case | Check |
|---|---|
| inventory-ok, password, hostkey-rotate | exit 0, valid JSON, hostname `ssh-<case>`, exactly one address, the target's own |
| exit-nonzero | exit 3 |
| bad-json | exit 0, output does not parse |
| timeout | still running after 8 s (`SSH_LAB_VERIFY_WAIT`) |
| oversized | exit 0, more than 65536 bytes, valid JSON |
| unknown-keys | exit 0, valid JSON with `hostname` and `favorite_color` |

It also checks that the `password` target refuses the key. Verify keeps its own host key pins in
`~/.config/scanopy-lab/ssh/known_hosts`, separate from Scanopy's.

The kernel does not let a host reach its own macvlan children, so an `ssh` from the VM to
`192.168.7.16x` fails even when the target is healthy. Verify from the Mac.

```sh
make ssh-lab-status
```

Checks on the VM, per target: link present, address assigned, unit active, listening on
`<ip>:2222`, plus the nftables table. Prints each host key fingerprint.

## Rotate a host key

```sh
make ssh-lab-rotate-hostkey IP=192.168.7.167
```

Replaces that target's host key, restarts its sshd, prints the old and new fingerprints, and
drops the address from verify's own `known_hosts` so `make ssh-lab-verify` keeps passing. Scanopy's
pin is left alone: that is the thing under test. Scan `.167` at least once before rotating, or
there is no pin to refuse against.

To accept the new key in Scanopy afterwards, clear the pin the way the feature provides.

## Create the credentials in Scanopy

1. Scan the lab network once without SSH credentials, so the eight targets exist as hosts (each
   has port 22 open).
2. Create one credential per case, assigned to that case's host (`host_assignments`). A
   network-wide assignment would run every script on every target.
3. Scan again and compare each host with the table above.

API: `POST /api/v1/credentials` with `Authorization: Bearer <api key>`. The generated schema
(`ui/src/lib/api/schema.d.ts`) names the route `/api/v1/credentials`, and secrets are
`{"mode": "Inline", "value": "..."}`. The `credential_type` fields are the `SshKey` and
`SshPassword` variants of `components["schemas"]["CredentialType"]`.

`make ssh-lab-deploy` installs every case script on the VM as
`/usr/local/lib/scanopy-lab/<case>.sh`, so the default script mode, a file on the scanned host,
names it by path. `daemon_os` is the OS of the daemon that reads the key (your Mac: `Unix`);
`target_os` is the scanned host's (the lab VM: `Unix`).

```sh
SCANOPY_URL="<server url>"
SCANOPY_API_KEY="<api key>"
ORG_ID="<organization id>"
HOST_ID="<id of the ssh-lab host for this case>"
CASE="inventory-ok"   # script name under tools/ssh/scripts/, without .sh

jq -n \
  --arg name "ssh-lab ${CASE} (key)" \
  --arg org "$ORG_ID" \
  --arg host "$HOST_ID" \
  --arg script "/usr/local/lib/scanopy-lab/${CASE}.sh" \
  --rawfile key ~/.config/scanopy-lab/ssh/id_ed25519 \
  '{
     name: $name,
     organization_id: $org,
     daemon_os: "Unix",
     credential_type: {
       type: "SshKey",
       username: "scanopy-ssh",
       private_key: {mode: "Inline", value: $key},
       script: {mode: "HostFile", path: $script},
       target_os: "Unix",
       timeout_seconds: 60
     },
     assigned_network_ids: [],
     host_assignments: [{host_id: $host, ip_address_ids: null}],
     tags: []
   }' |
curl -sS -X POST "$SCANOPY_URL/api/v1/credentials" \
  -H "Authorization: Bearer $SCANOPY_API_KEY" \
  -H "Content-Type: application/json" \
  --data-binary 10437 10437
```

The other two script modes, for the script-source tests:

- Text stored on the credential: `--rawfile script_text "tools/ssh/scripts/${CASE}.sh"` and
  `script: {mode: "Inline", value: $script_text}`.
- A file on the daemon's machine (the Mac): `script: {mode: "DaemonFile", path: "<absolute path
  to tools/ssh/scripts/${CASE}.sh in this checkout>"}`. A path starting with `~` is refused.

For the `password` target, swap the `credential_type` for:

```
credential_type: {
  type: "SshPassword",
  username: "scanopy-ssh",
  password: {mode: "Inline", value: $password},
  script: {mode: "HostFile", path: $script},
  target_os: "Unix",
  timeout_seconds: 60
}
```

with `--rawfile password ~/.config/scanopy-lab/ssh/password` in place of `--rawfile key ...`.
The password file ends without a newline, so `--rawfile` passes it unchanged. In the UI, paste the
file's contents into Password with "Enter value"; a file path there is read on the daemon's
machine, so it has to be absolute.

## Manage services

```sh
# On the VM
systemctl status 'sshd-test@*'
journalctl -u sshd-test@192.168.7.160 -f      # VERBOSE: every login, key fingerprint, failure
systemctl restart ssh-lab-network             # re-create links, addresses and the port-22 rule
nft list table ip scanopy_ssh_lab
```
