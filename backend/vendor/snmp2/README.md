<!--
====================================================================
SCANOPY VENDORED COPY — snmp2 0.5.0

Why vendored:
  * SNMPv3 needs a crypto backend. snmp2's `crypto-openssl` backend pulls in
    OpenSSL, which is banned in this repo (breaks the daemon's musl
    cross-compilation — see backend/src/tests/dependencies.rs).
  * snmp2's pure-Rust `crypto-rust` backend (0.5.0, and unreleased 0.5.1) does
    not compile: `Security::calculate_hmac` calls `mac.update(data)`, which is
    ambiguous between `hmac::Mac::update` and `digest::DynDigest::update` (both
    traits are imported). Reported/upstreamed separately.

Local patches (all marked in-source with `SCANOPY LOCAL PATCH`):
  * src/v3.rs `calculate_hmac` — `mac.update(data)` → `Mac::update(&mut mac, data)`
    (6 call sites, one per auth protocol). The compile fix this copy exists for.
  * src/asyncsession.rs — `drain_stale` before each send, and a request id burned
    the moment it is taken rather than when the response arrives. A request whose
    future was dropped mid-`recv` left the session permanently one answer behind,
    which surfaced as truncated tables on busy agents.
  * src/v3.rs — a `context_name` on `Security`, emitted as the scoped PDU's
    contextName in both the encrypted and plaintext branches of `build`. Upstream
    hard-codes it empty, so no caller could address a non-default context; Cisco
    IOS-XE keeps its per-VLAN bridge FDB in one. `build_init` still sends the null
    context, as engine discovery requires.
  * src/v3.rs `parse_v3` — RFC 3414 §3.2 step 7 timeliness (GH #813). Upstream
    read snmpEngineBoots = 0 as "not discovered" and rejected every authenticated
    message from an engine reporting it (`EngineBootsNotProvided`, now removed);
    MikroTik RouterOS always reports 0, which §2.2.1 allows. An explicit
    `time_synchronized` flag on `AuthoritativeState` now marks discovery. The
    message's boots and time are written to the cache only after its digest
    verifies, the boots = 2147483647 rule upstream lacked is enforced, and the
    window is RFC 7b's one-sided check against the extrapolated local time.
    `decrypt_aes` takes the message's boots and time for the IV (RFC 3826
    §3.1.4) instead of the cache.
  * src/asyncsession.rs, src/syncsession.rs `init` — RFC 3414 §4's second step:
    an authenticated GET with no varbinds, so a session leaves `init`
    time-synchronized even when the agent's discovery Report carried no clock.
    Without it the caller's first request drew a notInTimeWindow Report and an
    `AuthUpdated` error, which the daemon does not repeat.
  * src/v3.rs `parse_v3`, src/asyncsession.rs `init` — GH #846. A plaintext Report
    naming `usmStatsDecryptionErrors` returns `AuthErrorKind::DecryptionError`
    rather than the `NotAuthenticated` a wrong auth password gets, and `init` is
    split into `discover_engine` and `synchronize_time` (async session only).
    AES-256 under MD5 or SHA-1 has two key derivations (Blumenthal, Reeder); the
    daemon tries the second when an engine answers discovery and then cannot read
    the first encrypted request, which needs the two steps timed apart.
  * src/asyncsession.rs `send_and_recv`, src/pdu.rs `peek_response_id` and
    `is_earlier_request` — GH #848. After sending, the session reads until a
    datagram arrives that does not answer one of its own earlier requests, and
    passes over the ones that do. `drain_stale` removes only what arrived before
    the send; an agent that answered request N just after N+1 went out had that
    answer read as N+1's, failed with `RequestIdMismatch`, and left every later
    request reading the answer to the one before it. The id is the request-id for
    v1 and v2c and the plaintext msgID for v3, read before the message is
    authenticated. An id the session never issued is still `RequestIdMismatch`.
    The loop has no deadline of its own: the caller's timeout drops the future,
    as it always did. Async session only; the daemon does not use the sync one.
  * src/v3.rs `decrypt_des` (crypto-rust) — GH #846. DES padding is left in place
    instead of being stripped as PKCS#7. RFC 3414 makes it arbitrary and the
    scoped PDU's BER length bounds the read. net-snmp sends no padding when the
    scoped PDU is a multiple of 8 bytes, so about one reply in eight was refused
    as "Invalid padding" or lost its last byte, which broke table walks part way.

The first is the only one that blocks compilation. The others are behaviour
upstream does not offer, so returning to crates.io means re-applying them or
getting them upstreamed. Upstream 0.5.2 (2026-07-12) ships the `calculate_hmac`
fix and 0.5.1 a context name, but master still rejects boots = 0.

`cargo test --lib` in backend/ does not run this crate's tests (it has
dev-dependencies and is not a workspace member). Run them standalone:

  cargo test --manifest-path vendor/snmp2/Cargo.toml --target-dir <scratch> \
    --no-default-features --features crypto-rust,tokio,heap_buffers --lib
====================================================================
-->
<h2>
  RUST-SNMP
  <a href="https://crates.io/crates/snmp2"><img alt="crates.io page" src="https://img.shields.io/crates/v/snmp2.svg"></img></a>
  <a href="https://docs.rs/snmp2"><img alt="docs.rs page" src="https://docs.rs/snmp2/badge.svg"></img></a>
  <a href="https://github.com/roboplc/snmp2/actions/workflows/ci.yml">
    <img alt="GitHub Actions CI" src="https://github.com/roboplc/snmp2/actions/workflows/ci.yml/badge.svg"></img>
  </a>
</h2>

Dependency-free basic SNMP v1/v2/v3 client in Rust.

This is a fork of the original [snmp](https://crates.io/crates/snmp) crate
which has been abandoned long time ago.

SNMP2 is a part of [RoboPLC](https://www.roboplc.com) project.

New features added to the fork:

- SNMP v1 support (including v1 traps)
- SNMP v3 authentication (MD5, SHA1, SHA224, SHA256, SHA384, SHA512)
- SNMP v3 privacy (DES, AES128, AES192, AES256)
- MIBs support (requires `mibs` feature and `libnetsnmp` library installed)
- Async session (requires `tokio` feature)
- Crate code has been refactored and cleaned up
- OIDs have been migrated to
  [asn1](https://docs.rs/asn1-rs/latest/asn1_rs/struct.Oid.html)
- Improved PDU API, added trap handling examples

Supports:

- GET
- GETNEXT
- GETBULK
- SET
- Basic SNMP v1/v2 types
- Synchronous/Asynchronous requests
- UDP transport
- MIBs (with `mibs` feature, requires `libnetsnmp`)
- SNMP v3 (enable `crypto-openssl` or `crypto-rust` feature)

# Examples

## GET NEXT

```rust,no_run
use std::time::Duration;
use snmp2::{SyncSession, Value, Oid};

let sys_descr_oid = Oid::from(&[1,3,6,1,2,1,1,1,]).unwrap();
let agent_addr    = "198.51.100.123:161";
let community     = b"f00b4r";
let timeout       = Duration::from_secs(2);

let mut sess = SyncSession::new_v2c(agent_addr, community, Some(timeout), 0).unwrap();
let mut response = sess.getnext(&sys_descr_oid).unwrap();
if let Some((_oid, Value::OctetString(sys_descr))) = response.varbinds.next() {
    println!("myrouter sysDescr: {}", String::from_utf8_lossy(sys_descr));
}
```

## GET BULK

```rust,no_run
use std::time::Duration;
use snmp2::{SyncSession, Oid};

let system_oid      = Oid::from(&[1,3,6,1,2,1,1,]).unwrap();
let agent_addr      = "[2001:db8:f00:b413::abc]:161";
let community       = b"f00b4r";
let timeout         = Duration::from_secs(2);
let non_repeaters   = 0;
let max_repetitions = 7; // number of items in "system" OID

let mut sess = SyncSession::new_v2c(agent_addr, community, Some(timeout), 0).unwrap();
let response = sess.getbulk(&[&system_oid], non_repeaters, max_repetitions).unwrap();

for (name, val) in response.varbinds {
    println!("{} => {:?}", name, val);
}
```

## SET

```rust,no_run
use std::time::Duration;
use snmp2::{SyncSession, Value, Oid};

let syscontact_oid  = Oid::from(&[1,3,6,1,2,1,1,4,0]).unwrap();
let contact         = Value::OctetString(b"Thomas A. Anderson");
let agent_addr      = "[2001:db8:f00:b413::abc]:161";
let community       = b"f00b4r";
let timeout         = Duration::from_secs(2);

let mut sess = SyncSession::new_v2c(agent_addr, community, Some(timeout), 0).unwrap();
let response = sess.set(&[(&syscontact_oid, contact)]).unwrap();

assert_eq!(response.error_status, snmp2::snmp::ERRSTATUS_NOERROR);
for (name, val) in response.varbinds {
    println!("{} => {:?}", name, val);
}
```

## TRAPS

```rust,no_run
use std::net::UdpSocket;
use snmp2::Pdu;

let socket = UdpSocket::bind("0.0.0.0:1162").expect("Could not bind socket");
loop {
    let mut buf = [0; 1500];
    let size = socket.recv(&mut buf).expect("Could not receive data");
    let data = &buf[..size];
    let pdu = Pdu::from_bytes(data).expect("Could not parse PDU");
    println!("Version: {}", pdu.version().unwrap());
    println!("Community: {}", std::str::from_utf8(pdu.community).unwrap());
    for (name, value) in pdu.varbinds {
        println!("{}={:?}", name, value);
    }
}
```

## PDU to Bytes Conversion

Convert PDU structures to byte arrays for UDP communication:

```rust,ignore
use snmp2::{Pdu, Oid, Version};
use std::net::UdpSocket;

// Parse a received PDU
let received_pdu = Pdu::from_bytes(&received_data).unwrap();

// Convert PDU back to bytes for forwarding or storage
let bytes = received_pdu.to_bytes().unwrap();

// Send via UDP socket
let socket = UdpSocket::bind("0.0.0.0:1161").unwrap();
socket.send_to(&bytes, target_addr).unwrap();
```

### With SNMPv3 (enable `crypto-openssl` or `crypto-rust`)

When using SNMPv3, you need to provide the security context to convert the PDU to bytes:

```rust,ignore
#[cfg(feature = "v3")]
{
    use snmp2::{Pdu, v3};

    // Setup security parameters (Authentication and Privacy)
    let security = v3::Security::new(b"public", b"secure")
        .with_auth_protocol(v3::AuthProtocol::Sha1)
        .with_auth(v3::Auth::AuthPriv {
            cipher: v3::Cipher::Aes128,
            privacy_password: b"privacy_password".to_vec(),
        })
        .with_engine_id(&[0x80, 0x00, 0x00, 0x00, 0x01])
        .unwrap();

    // Parse a received V3 PDU
    // Note: You need a mutable reference to security to update authoritative state if needed
    let mut security_parse = security.clone();
    let received_pdu = Pdu::from_bytes_with_security(&received_data, Some(&mut security_parse)).unwrap();

    // Convert V3 PDU back to bytes
    // This uses the security context to encrypt and sign the PDU
    let bytes = received_pdu.to_bytes_with_security(Some(&security)).unwrap();
}
```

## Async session

```rust,no_run
use std::time::Duration;
use snmp2::{AsyncSession, Value, Oid};

async fn get_next() {
    // timeouts should be handled by the caller with `tokio::time::timeout`
    let sys_descr_oid = Oid::from(&[1,3,6,1,2,1,1,1,]).unwrap();
    let agent_addr    = "198.51.100.123:161";
    let community     = b"f00b4r";
    let mut sess = AsyncSession::new_v2c(agent_addr, community, 0).await.unwrap();
    let mut response = sess.getnext(&sys_descr_oid).await.unwrap();
    if let Some((_oid, Value::OctetString(sys_descr))) = response.varbinds.next() {
        println!("myrouter sysDescr: {}", String::from_utf8_lossy(sys_descr));
    }
}
```

## Working with MIBs

Prepare the system

```shell
apt-get install libsnmp-dev snmp-mibs-downloader
```

```rust,ignore
use snmp2::{mibs::{self, MibConversion as _}, Oid};

mibs::init(&mibs::Config::new().mibs(&["./ibmConvergedPowerSystems.mib"]))
    .unwrap();
let snmp_oid = Oid::from(&[1, 3, 6, 1, 4, 1, 2, 6, 201, 3]).unwrap();
let name = snmp_oid.mib_name().unwrap();
assert_eq!(name, "IBM-CPS-MIB::cpsSystemSendTrap");
let snmp_oid2 = Oid::from_mib_name(&name).unwrap();
assert_eq!(snmp_oid, snmp_oid2);
```

# SNMPv3

- Requires enabling one of the features: `crypto-openssl` or `crypto-rust`.
  - `crypto-openssl`: uses OpenSSL for hashing/HMAC and symmetric encryption.
  - `crypto-rust`: uses pure Rust crypto crates for hashing/HMAC and encryption.

- Cryptographic algorithms are provided by the selected backend:
  - `crypto-openssl`: [openssl](https://www.openssl.org/)
  - `crypto-rust`: pure Rust crates [Rust Crypto](https://github.com/RustCrypto): (`md-5`, `sha1`, `sha2`, `hmac`, `aes`, `des`, etc.)

- For authentication, supports: MD5 (RFC3414), SHA1 (RFC3414) and non-standard
  SHA224, SHA256, SHA384, SHA512.

- For privacy, supports: DES (RFC3414), AES128-CFB (RFC3826) and non-standard
  AES192-CFB, AES256-CFB. Additional/different AES modes are not supported and
  may require patching the crate.

Note: For `crypto-openssl`, DES legacy encryption may be disabled in OpenSSL by default
or not supported at all. Refer to the library documentation how to enable it.

Note: if both `crypto-openssl` and `crypto-rust` features are enabled,
`crypto-openssl` will have higher priority and will be used as the crypto
backend.

### Feature selection examples

Pure Rust backend:

```shell
cargo add snmp2 --features crypto-rust
```

OpenSSL backend (Windows-friendly vendored build):

```shell
cargo add snmp2 --features "crypto-openssl,openssl/vendored"
```

## Example

Authentication: SHA1, encryption: AES128-CFB

```rust,no_run
use snmp2::{SyncSession, v3, Oid};
use std::time::Duration;

// the security parameters also keep authoritative engine ID and boot/time
// counters. these can be either set or resolved/updated automatically.
let security = v3::Security::new(b"public", b"secure")
    .with_auth_protocol(v3::AuthProtocol::Sha1)
    .with_auth(v3::Auth::AuthPriv {
        cipher: v3::Cipher::Aes128,
        privacy_password: b"secure-encrypt".to_vec(),
    });
let mut sess =
    SyncSession::new_v3("192.168.1.1:161", Some(Duration::from_secs(2)), 0, security).unwrap();
// In case if engine_id is not provided in security parameters, it is necessary
// to call init() method to send a blank unauthenticated request to the target
// to get the engine_id.
sess.init().unwrap();
loop {
    let res = match sess.get(&Oid::from(&[1, 3, 6, 1, 2, 1, 1, 3, 0]).unwrap()) {
        Ok(r) => r,
        // In case if the engine boot / time counters are not set in the security parameters or
        // they have been changed on the target, e.g. after a reboot, the session returns
        // an error with the AuthUpdated code. In this case, security parameters are automatically
        // updated and the request should be repeated.
        Err(snmp2::Error::AuthUpdated) => continue,
        Err(e) => panic!("{}", e),
    };
    println!("{} {:?}", res.version().unwrap(), res.varbinds);
    std::thread::sleep(Duration::from_secs(1));
}
```

## Building (`crypto-openssl`)

When using the `crypto-openssl` backend, in case of problems (e.g. with
[cross-rs](https://github.com/cross-rs/cross)), add `openssl` with `vendored` feature:

```shell
cargo add openssl --features vendored
```

## FIPS-140 support (`crypto-openssl`)

When using the `crypto-openssl` backend, the crate becomes FIPS-140 compliant as soon
as FIPS mode is activated in `openssl`. Refer to the
[openssl crate](https://docs.rs/openssl) crate and
[openssl library](https://www.openssl.org/) documentation for more details.
The `crypto-rust` backend does not rely on OpenSSL and is not FIPS-certified.

## MSRV

1.83.0

## Copyright

Copyright 2016-2018 Hroi Sigurdsson

Copyright 2024 Serhij Symonenko, [Bohemia Automation Limited](https://www.bohemia-automation.com)

Licensed under the [Apache License, Version
2.0](http://www.apache.org/licenses/LICENSE-2.0) or the [MIT
license](http://opensource.org/licenses/MIT), at your option. This file may not
be copied, modified, or distributed except according to those terms.
