// Copyright (c) 2015-2016 Robert Clipsham <robert@octarineparrot.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Support for sending and receiving data link layer packets using the netmap library.

#![allow(bad_style)]

use self::netmap_sys::netmap::{netmap_slot, nm_ring_empty};
use self::netmap_sys::netmap_user::{
    nm_close, nm_desc, nm_nextpkt, nm_open, nm_pkthdr, nm_ring_next, NETMAP_BUF, NETMAP_FD,
    NETMAP_TXRING,
};
use Channel::Ethernet;
use crate::{DataLinkReceiver, DataLinkSender, NetworkInterface};

use std::ffi::CString;
use std::fs::File;
use std::io;
use std::io::Read;
use std::mem;
use std::path::Path;
use std::ptr;
use std::slice;
use std::sync::Arc;
use std::time::Duration;

#[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
#[repr(C)]
struct pollfd {
    fd: libc::c_int,
    events: libc::c_short,
    revents: libc::c_short,
}

#[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
const POLLIN: libc::c_short = 0x0001;
#[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
const POLLOUT: libc::c_short = 0x0004;

#[cfg(target_os = "freebsd")]
type nfds_t = libc::c_uint;
#[cfg(any(target_os = "linux", target_os = "android"))]
type nfds_t = libc::c_ulong;

extern "C" {
    fn ppoll(
        fds: *mut pollfd,
        nfds: nfds_t,
        timeout: *const libc::timespec,
        newsigmask: *const libc::sigset_t,
    ) -> libc::c_int;
}

struct NmDesc {
    desc: *mut nm_desc,
    buf_size: libc::c_uint,
}

unsafe impl Send for NmDesc {}
unsafe impl Sync for NmDesc {}

impl NmDesc {
    fn new(iface: &NetworkInterface) -> io::Result<NmDesc> {
        let ifname = CString::new(("netmap:".to_owned() + &iface.name[..]).as_bytes());
        let desc = unsafe { nm_open(ifname.unwrap().as_ptr(), ptr::null(), 0, ptr::null()) };

        if desc.is_null() {
            Err(io::Error::last_os_error())
        } else {
            let mut f = try!(File::open(&Path::new(
                "/sys/module/netmap/parameters/buf_size"
            )));
            let mut num_str = String::new();
            try!(f.read_to_string(&mut num_str));
            let buf_size = num_str.trim_right().parse().unwrap();

            Ok(NmDesc {
                desc: desc,
                buf_size: buf_size,
            })
        }
    }
}

impl Drop for NmDesc {
    fn drop(&mut self) {
        unsafe {
            nm_close(self.desc);
        }
    }
}

/// The netmap's specific configuration.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Config {
    /// The read timeout. Defaults to None.
    pub read_timeout: Option<Duration>,

    /// The write timeout. Defaults to None.
    pub write_timeout: Option<Duration>,
}

impl<'a> From<&'a super::Config> for Config {
    fn from(config: &super::Config) -> Config {
        Config {
            read_timeout: config.read_timeout,
            write_timeout: config.write_timeout,
        }
    }
}

impl Default for Config {
    fn default() -> Config {
        Config {
            read_timeout: None,
            write_timeout: None,
        }
    }
}

#[inline]
fn get_timeout(to: Option<Duration>) -> Option<libc::timespec> {
    to.map(|dur| libc::timespec {
        tv_sec: dur.as_secs() as libc::time_t,
        tv_nsec: dur.subsec_nanos() as libc::c_long,
    })
}

/// Create a datalink channel using the netmap library.
#[inline]
pub fn channel(network_interface: &NetworkInterface, config: Config) -> io::Result<super::Channel> {
    // FIXME probably want one for each of send/recv
    let desc = NmDesc::new(network_interface);
    match desc {
        Ok(desc) => {
            let arc = Arc::new(desc);

            Ok(Ethernet(
                Box::new(DataLinkSenderImpl {
                    desc: arc.clone(),
                    timeout: get_timeout(config.write_timeout),
                }),
                Box::new(DataLinkReceiverImpl {
                    desc: arc,
                    timeout: get_timeout(config.read_timeout),
                }),
            ))
        }
        Err(e) => Err(e),
    }
}

struct DataLinkSenderImpl {
    desc: Arc<NmDesc>,
    timeout: Option<libc::timespec>,
}

impl DataLinkSender for DataLinkSenderImpl {
    #[inline]
    fn build_and_send(
        &mut self,
        num_packets: usize,
        packet_size: usize,
        func: &mut FnMut(&mut [u8]),
    ) -> Option<io::Result<()>> {
        assert!(packet_size <= self.desc.buf_size as usize);
        let desc = self.desc.desc;
        let mut fds = pollfd {
            fd: unsafe { NETMAP_FD(desc) },
            events: POLLOUT,
            revents: 0,
        };
        let mut packet_idx = 0usize;
        while packet_idx < num_packets {
            unsafe {
                let timespec = self
                    .timeout
                    .as_ref()
                    .map(|ts| ts as *const _)
                    .unwrap_or(ptr::null());
                if ppoll(&mut fds, 1, timespec, ptr::null()) < 0 {
                    return Some(Err(io::Error::last_os_error()));
                }
                let ring = NETMAP_TXRING((*desc).nifp, 0);
                while !nm_ring_empty(ring) && packet_idx < num_packets {
                    let i = (*ring).cur;
                    let slot_ptr: *mut netmap_slot = mem::transmute(&mut (*ring).slot);
                    let buf = NETMAP_BUF(ring, (*slot_ptr.offset(i as isize)).buf_idx as isize);
                    let slice = slice::from_raw_parts_mut(buf as *mut u8, packet_size);
                    (*slot_ptr.offset(i as isize)).len = packet_size as u16;
                    func(slice);
                    let next = nm_ring_next(ring, i);
                    (*ring).head = next;
                    (*ring).cur = next;
                    packet_idx += 1;
                }
            }
        }

        Some(Ok(()))
    }

    #[inline]
    fn send_to(&mut self, packet: &[u8], _dst: Option<NetworkInterface>) -> Option<io::Result<()>> {
        self.build_and_send(1, packet.len(), &mut |eh: &mut [u8]| {
            eh.clone_from_slice(packet);
        })
    }
}

struct DataLinkReceiverImpl {
    desc: Arc<NmDesc>,
    timeout: Option<libc::timespec>,
}

impl DataLinkReceiver for DataLinkReceiverImpl {
    fn next(&mut self) -> io::Result<&[u8]> {
        let desc = self.desc.desc;
        let mut h: nm_pkthdr = unsafe { mem::uninitialized() };
        let mut buf = unsafe { nm_nextpkt(desc, &mut h) };
        if buf.is_null() {
            let mut fds = pollfd {
                fd: unsafe { NETMAP_FD(desc) },
                events: POLLIN,
                revents: 0,
            };
            let timespec = self
                .timeout
                .as_ref()
                .map(|ts| ts as *const _)
                .unwrap_or(ptr::null());
            if unsafe { ppoll(&mut fds, 1, timespec, ptr::null()) } < 0 {
                return Err(io::Error::last_os_error());
            }
            buf = unsafe { nm_nextpkt(desc, &mut h) };
        }
        Ok(unsafe { slice::from_raw_parts(buf, h.len as usize) })
    }
}

/// Get a list of available network interfaces for the current machine.
pub fn interfaces() -> Vec<NetworkInterface> {
    #[path = "unix_interfaces.rs"]
    mod interfaces;
    interfaces::interfaces()
}
