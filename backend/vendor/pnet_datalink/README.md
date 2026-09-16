<!--
====================================================================
SCANOPY VENDORED COPY — pnet_datalink 0.35.0

Why vendored:
  * macOS/BSD's `bpf.rs` backend calls `libc::FD_SET(fd, &mut fd_set)` unconditionally during
    `datalink::channel()` construction (lines 239, 255) and again on every send/receive (291, 336,
    386). `fd_set` is a fixed `FD_SETSIZE`-slot bitmap (1024 on Darwin) with no bounds check —
    `FD_SET` on an fd >= 1024 is an out-of-bounds write, which panics somewhere Rust unwinding
    isn't permitted, aborting the whole process (SIGABRT, no logged panic line). This daemon
    raises its fd soft limit to 10,240 at startup for scan concurrency, so under real scan load an
    fd >= 1024 reaching a datalink channel open is expected, not an edge case.
  * `linux.rs` already fixed this identical bug class upstream: libpnet issues #612 and #639 are
    the same `FD_SET`-on-large-fd panic, and PR #681 (merged) replaced `select`/`FD_SET` with
    `poll` in `linux.rs` only. `bpf.rs` (macOS/BSD) was never patched. `winpcap.rs` (Windows) uses
    the WinPcap/Npcap C API directly and has no `FD_SET`/`select` at all — unaffected by
    construction, not by luck.

Local patches (all marked in-source with `SCANOPY LOCAL PATCH`):
  * src/bpf.rs — the same `select`/`FD_SET` → `poll` rewrite `linux.rs` already has upstream,
    ported to all five call sites (channel construction x2, `build_and_send`, `send_to`, `next`).
    Each `DataLinkSenderImpl`/`DataLinkReceiverImpl` only ever monitors one fd, so `poll`'s
    array-of-one is a direct, behavior-preserving replacement for `select`'s fixed bitmap — the
    existing `pselect` calls already pass a null sigmask, so nothing here depends on `pselect`'s
    signal-atomicity property over `poll`. Timeout conversion mirrors `linux.rs`'s own
    `tv_sec * 1000 + tv_nsec / 1_000_000` (`-1` for block-forever). One deliberate departure from
    `linux.rs`'s pattern: `linux.rs` narrows on `revents & POLLOUT/POLLIN != 0` after a positive
    `poll()` return, because that's meaningful for its AF_PACKET socket. This `/dev/bpf` character
    device does not reliably set that bit even when the read/write below succeeds — confirmed
    live, an early version of this patch that added the same narrowing check produced
    "Unexpected poll event" on essentially every send. The `select`/`FD_SET` code this replaces
    never checked `FD_ISSET` either; it trusted a positive return alone and always proceeded. All
    three call sites match that: `ret > 0` proceeds, regardless of which bits `revents` carries.
  * src/bpf.rs — a debug-level log of the fd handed to each channel at construction, kept
    permanently (not a temporary diagnostic): the one place that actually sees the raw fd, so any
    future recurrence of this bug class is a logged fact instead of a silent abort.

Remove this vendored copy and return to a crates.io dependency once upstream ships a release of
`pnet_datalink` with the equivalent fix in `bpf.rs` (no such release exists as of this writing —
`bpf.rs` in every published 0.35.x is still the unpatched `select`/`FD_SET` version).
====================================================================
-->
# libpnet [![Crates.io](https://img.shields.io/crates/v/pnet.svg)](https://crates.io/crates/pnet) ![License](https://img.shields.io/crates/l/pnet.svg) [![Documentation](https://docs.rs/pnet/badge.svg)](https://docs.rs/pnet/)

Build Status: [![Build Status](https://github.com/libpnet/libpnet/actions/workflows/ci.yml/badge.svg)](https://github.com/libpnet/libpnet/actions/workflows/ci.yml)

Discussion and support:

 * Live chat on IRC - [#libpnet on irc.libera.chat](https://kiwiirc.com/nextclient/irc.libera.chat/libpnet?nick=pnet-user42)
 * [GitHub Discussions](https://github.com/libpnet/libpnet/discussions)

`libpnet` provides a cross-platform API for low level networking using Rust.

There are four key components:

 * The `packet` module, allowing safe construction and manipulation of packets;
 * The `pnet_macros` crate, providing infrastructure for the packet module;
 * The `transport` module, which allows implementation of transport protocols;
 * The `datalink` module, which allows sending and receiving data link packets directly.

## Why?

There are lots of reasons to use low level networking, and many more to do it using Rust. A few are
outlined here:

### Developing Transport Protocols

There are usually two ways to go about developing a new transport layer protocol:

 * Write it in a scripting language such as Python;
 * Write it using C.

The former is great for trying out new ideas and rapid prototyping, however not so great as a
real-world implementation. While you can usually get reasonable performance out of these
implementations, they're generally significantly slower than an implementation in C, and not
suitable for any "heavy lifting".

The next option is to write it in C - this will give you great performance, but comes with a number
of other issues:

 * Lack of memory safety - this is a huge source of security vulnerabilities and other bugs in
   C-based network stacks. It is far too easy to forget a bounds check or use a pointer after it is
   freed.
 * Lack of thread safety - you have to be very careful to make sure the correct locks are used, and
   used correctly.
 * Lack of high level abstractions - part of the appeal of scripting languages such as Python is
   the higher level of abstraction which enables simpler APIs and ease of programming.

Using `libpnet` and Rust, you get the best of both worlds. The higher level abstractions, memory
and thread safety, alongside the performance of C.

### Network Utilities

Many networking utilities such as ping and traceroute rely on being able to manipulate network and
transport headers, which isn't possible with standard networking stacks such as those provided by
`std::io::net`.

### Data Link Layer

It can be useful to work directly at the data link layer, to see packets as they are "on the wire".
There are lots of uses for this, including network diagnostics, packet capture and traffic shaping.

## Documentation

API documentation for the latest build can be found here: https://docs.rs/pnet/

## Usage

To use `libpnet` in your project, add the following to your Cargo.toml:

```
[dependencies.pnet]
version = "0.35.0"
```

`libpnet` should work with the latest stable version of Rust.

When running the test suite, there are a number of networking tests which will
likely fail - the easiest way to workaround this is to run `cargo test` as a
root or administrative user. This can often be avoided, however it is more
involved.

### Windows

There are three requirements for building on Windows:

 * You must use a version of Rust which uses the MSVC toolchain
 * You must have [WinPcap](https://www.winpcap.org/) or [npcap](https://nmap.org/npcap/) installed
   (tested with version WinPcap 4.1.3) (If using npcap, make sure to install with the "Install Npcap in WinPcap API-compatible Mode")
 * You must place `Packet.lib` from the [WinPcap Developers pack](https://www.winpcap.org/devel.htm)
   in a directory named `lib`, in the root of this repository. Alternatively, you can use any of the
   locations listed in the `%LIB%`/`$Env:LIB` environment variables. For the 64 bit toolchain it is
   in `WpdPack/Lib/x64/Packet.lib`, for the 32 bit toolchain, it is in `WpdPack/Lib/Packet.lib`.
