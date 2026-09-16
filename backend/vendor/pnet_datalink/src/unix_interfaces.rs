// Copyright (c) 2014-2016 Robert Clipsham <robert@octarineparrot.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Interface listing implementation for all non-Windows platforms.

use crate::{MacAddr, NetworkInterface};

use ipnetwork::{ip_mask_to_prefix, IpNetwork};
use pnet_sys;

use libc;

use std::ffi::{CStr, CString};
use std::mem;
use std::mem::MaybeUninit;
use std::net::IpAddr;
use std::os::raw::c_char;
use std::str::from_utf8_unchecked;

/// Get a list of available network interfaces for the current machine.
pub fn interfaces() -> Vec<NetworkInterface> {
    fn merge(old: &mut NetworkInterface, new: &NetworkInterface) {
        old.mac = match new.mac {
            None => old.mac,
            _ => new.mac,
        };
        old.ips.extend_from_slice(&new.ips[..]);
        old.flags = old.flags | new.flags;
    }

    let mut ifaces: Vec<NetworkInterface> = Vec::new();
    let mut addrs: MaybeUninit<*mut libc::ifaddrs> = MaybeUninit::uninit();

    // Safety: addrs.as_mut_ptr() is valid, it points to addrs.
    if unsafe { libc::getifaddrs(addrs.as_mut_ptr()) } != 0 {
        return ifaces;
    }

    // Safety: If there was an error, we would have already returned.
    // Therefore, getifaddrs has initialized `addrs`.
    let addrs = unsafe { addrs.assume_init() };

    let mut addr = addrs;
    while !addr.is_null() {
        // Safety: We assume that addr is valid for the lifetime of this loop
        // body, and is not mutated.
        let addr_ref: &libc::ifaddrs = unsafe {&*addr};

        let c_str = addr_ref.ifa_name as *const c_char;

        // Safety: ifa_name is a null terminated interface name
        let bytes = unsafe { CStr::from_ptr(c_str).to_bytes() };

        // Safety: Interfaces on unix must be valid UTF-8
        // TODO: Really? They *must* be UTF-8?
        let name = unsafe {from_utf8_unchecked(bytes).to_owned() };
        let (mac, ip) = sockaddr_to_network_addr(addr_ref.ifa_addr as *const libc::sockaddr);
        let (_, netmask) = sockaddr_to_network_addr(addr_ref.ifa_netmask as *const libc::sockaddr);
        let prefix = netmask
            .and_then(|netmask| ip_mask_to_prefix(netmask).ok())
            .unwrap_or(0);
        let network = ip.and_then(|ip| IpNetwork::new(ip, prefix).ok());
        let ni = NetworkInterface {
            name: name.clone(),
            description: "".to_string(),
            index: 0,
            mac: mac,
            ips: network.into_iter().collect(),
            flags: addr_ref.ifa_flags,
        };
        let mut found: bool = false;
        for iface in &mut ifaces {
            if name == iface.name {
                merge(iface, &ni);
                found = true;
            }
        }
        if !found {
            ifaces.push(ni);
        }

        addr = addr_ref.ifa_next;
    }

    // Safety: addrs has been previously allocated through getifaddrs
    unsafe {
        libc::freeifaddrs(addrs);
    }

    for iface in &mut ifaces {
        let name = CString::new(iface.name.as_bytes()).unwrap();

        // Safety: name.as_ptr() is a valid pointer
        unsafe {
            iface.index = libc::if_nametoindex(name.as_ptr());
        }
    }

    ifaces
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn sockaddr_to_network_addr(sa: *const libc::sockaddr) -> (Option<MacAddr>, Option<IpAddr>) {
    use std::net::SocketAddr;

    unsafe {
        if sa.is_null() {
            (None, None)
        } else if (*sa).sa_family as libc::c_int == libc::AF_PACKET {
            let sll: *const libc::sockaddr_ll = mem::transmute(sa);
            let mac = MacAddr(
                (*sll).sll_addr[0],
                (*sll).sll_addr[1],
                (*sll).sll_addr[2],
                (*sll).sll_addr[3],
                (*sll).sll_addr[4],
                (*sll).sll_addr[5],
            );

            (Some(mac), None)
        } else {
            let addr = pnet_sys::sockaddr_to_addr(
                mem::transmute(sa),
                mem::size_of::<libc::sockaddr_storage>(),
            );

            match addr {
                Ok(SocketAddr::V4(sa)) => (None, Some(IpAddr::V4(*sa.ip()))),
                Ok(SocketAddr::V6(sa)) => (None, Some(IpAddr::V6(*sa.ip()))),
                Err(_) => (None, None),
            }
        }
    }
}

#[cfg(any(
    target_os = "openbsd",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "illumos",
    target_os = "solaris",
    target_os = "macos",
    target_os = "ios",
    target_os = "tvos"
))]
fn sockaddr_to_network_addr(sa: *const libc::sockaddr) -> (Option<MacAddr>, Option<IpAddr>) {
    use crate::bindings::bpf;
    use std::net::SocketAddr;

    unsafe {
        if sa.is_null() {
            (None, None)
        } else if (*sa).sa_family as libc::c_int == bpf::AF_LINK {
            let sdl: *const bpf::sockaddr_dl = mem::transmute(sa);
            let nlen = (*sdl).sdl_nlen as usize;
            let mac = MacAddr(
                (*sdl).sdl_data[nlen] as u8,
                (*sdl).sdl_data[nlen + 1] as u8,
                (*sdl).sdl_data[nlen + 2] as u8,
                (*sdl).sdl_data[nlen + 3] as u8,
                (*sdl).sdl_data[nlen + 4] as u8,
                (*sdl).sdl_data[nlen + 5] as u8,
            );

            (Some(mac), None)
        } else {
            let addr = pnet_sys::sockaddr_to_addr(
                mem::transmute(sa),
                mem::size_of::<libc::sockaddr_storage>(),
            );

            match addr {
                Ok(SocketAddr::V4(sa)) => (None, Some(IpAddr::V4(*sa.ip()))),
                Ok(SocketAddr::V6(sa)) => (None, Some(IpAddr::V6(*sa.ip()))),
                Err(_) => (None, None),
            }
        }
    }
}
