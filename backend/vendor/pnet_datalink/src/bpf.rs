// Copyright (c) 2014-2016 Robert Clipsham <robert@octarineparrot.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Support for sending and receiving data link layer packets using the /dev/bpf device.

use crate::bindings::bpf;
use crate::{DataLinkReceiver, DataLinkSender, NetworkInterface};

use pnet_sys;

use std::collections::VecDeque;
use std::ffi::CString;
use std::io;
use std::mem::{self, align_of};
use std::sync::Arc;
use std::time::Duration;

static ETHERNET_HEADER_SIZE: usize = 14;
static NULL_HEADER_SIZE: usize = 4;

/// The BPF-specific configuration.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Config {
    /// The size of buffer to use when writing packets. Defaults to 4096.
    pub write_buffer_size: usize,

    /// The size of buffer to use when reading packets. Defaults to 4096.
    pub read_buffer_size: usize,

    /// The read timeout. Defaults to None.
    pub read_timeout: Option<Duration>,

    /// The write timeout. Defaults to None.
    pub write_timeout: Option<Duration>,

    /// The number of /dev/bpf* file descriptors to attempt before failing.
    ///
    /// This setting is only used on OS X - FreeBSD uses a single /dev/bpf rather than creating a
    /// new descriptor each time one is opened.
    ///
    /// Defaults to: 1000.
    pub bpf_fd_attempts: usize,
}

impl<'a> From<&'a super::Config> for Config {
    fn from(config: &super::Config) -> Config {
        Config {
            write_buffer_size: config.write_buffer_size,
            read_buffer_size: config.read_buffer_size,
            bpf_fd_attempts: config.bpf_fd_attempts,
            read_timeout: config.read_timeout,
            write_timeout: config.write_timeout,
        }
    }
}

impl Default for Config {
    fn default() -> Config {
        Config {
            write_buffer_size: 4096,
            read_buffer_size: 4096,
            bpf_fd_attempts: 1000,
            read_timeout: None,
            write_timeout: None,
        }
    }
}

/// Create a datalink channel using the /dev/bpf device
// NOTE buffer must be word aligned.
#[inline]
pub fn channel(network_interface: &NetworkInterface, config: Config) -> io::Result<super::Channel> {
    #[cfg(any(
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "illumos",
        target_os = "solaris"
    ))]
    fn get_fd(_attempts: usize) -> libc::c_int {
        let c_file_name = CString::new(&b"/dev/bpf"[..]).unwrap();
        unsafe {
            libc::open(
                c_file_name.as_ptr(),
                libc::O_RDWR,
                0,
            )
        }
    }

    #[cfg(any(target_os = "openbsd", target_os = "macos", target_os = "ios", target_os = "tvos"))]
    fn get_fd(attempts: usize) -> libc::c_int {
        for i in 0..attempts {
            let fd = unsafe {
                let file_name = format!("/dev/bpf{}", i);
                let c_file_name = CString::new(file_name.as_bytes()).unwrap();
                libc::open(
                    c_file_name.as_ptr(),
                    libc::O_RDWR,
                    0,
                )
            };
            if fd != -1 {
                return fd;
            }
        }

        -1
    }

    #[cfg(any(
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "illumos",
        target_os = "solaris"
    ))]
    fn set_feedback(fd: libc::c_int) -> io::Result<()> {
        if unsafe { bpf::ioctl(fd, bpf::BIOCFEEDBACK, &1) } == -1 {
            let err = io::Error::last_os_error();
            unsafe {
                libc::close(fd);
            }
            return Err(err);
        }
        Ok(())
    }

    #[cfg(any(target_os = "macos", target_os = "openbsd", target_os = "ios", target_os = "tvos"))]
    fn set_feedback(_fd: libc::c_int) -> io::Result<()> {
        Ok(())
    }

    let fd = get_fd(config.bpf_fd_attempts);
    if fd == -1 {
        return Err(io::Error::last_os_error());
    }
    let mut iface: bpf::ifreq = unsafe { mem::zeroed() };
    for (i, c) in network_interface.name.bytes().enumerate() {
        iface.ifr_name[i] = c as libc::c_char;
    }

    let buflen = config.read_buffer_size as libc::c_uint;
    // NOTE Buffer length must be set before binding to an interface
    //      otherwise this will return Invalid Argument
    if unsafe { bpf::ioctl(fd, bpf::BIOCSBLEN, &buflen) } == -1 {
        let err = io::Error::last_os_error();
        unsafe {
            libc::close(fd);
        }
        return Err(err);
    }

    // Set the interface to use
    if unsafe { bpf::ioctl(fd, bpf::BIOCSETIF, &iface) } == -1 {
        let err = io::Error::last_os_error();
        unsafe {
            libc::close(fd);
        }
        return Err(err);
    }

    // Return from read as soon as packets are available - don't wait to fill the
    // buffer
    if unsafe { bpf::ioctl(fd, bpf::BIOCIMMEDIATE, &1) } == -1 {
        let err = io::Error::last_os_error();
        unsafe {
            libc::close(fd);
        }
        return Err(err);
    }

    // Get the device type
    let mut dlt: libc::c_uint = 0;
    if unsafe { bpf::ioctl(fd, bpf::BIOCGDLT, &mut dlt) } == -1 {
        let err = io::Error::last_os_error();
        unsafe {
            libc::close(fd);
        }
        return Err(err);
    }

    let mut loopback = false;
    let mut buffer_offset = 0;
    let mut allocated_read_buffer_size = config.read_buffer_size;
    // The loopback device does weird things
    // FIXME This should really just be another L2 packet header type
    if dlt == bpf::DLT_NULL {
        loopback = true;
        // The loopback device provides a smaller (4-byte) header than ethernet (14-byte).
        // We deal with this by offsetting the write buffer, then overwriting the null header
        // with a zeroed ethernet header. This is complicated by the fact that the buffer
        // offset must be a multiple of four for pointer alignment, and that the write itself
        // must be 4096 bytes.
        let align = align_of::<bpf::bpf_hdr>();
        buffer_offset = (ETHERNET_HEADER_SIZE - NULL_HEADER_SIZE).next_multiple_of(align);
        allocated_read_buffer_size += buffer_offset;

        // Allow packets to be read back after they are written
        if let Err(e) = set_feedback(fd) {
            return Err(e);
        }
    } else {
        // Don't fill in source MAC
        if unsafe { bpf::ioctl(fd, bpf::BIOCSHDRCMPLT, &1) } == -1 {
            let err = io::Error::last_os_error();
            unsafe {
                libc::close(fd);
            }
            return Err(err);
        }
    }

    // Enable nonblocking
    if unsafe { libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK) } == -1 {
        let err = io::Error::last_os_error();
        unsafe {
            pnet_sys::close(fd);
        }
        return Err(err);
    }

    // SCANOPY LOCAL PATCH: this used to also FD_ZERO/FD_SET a `fd_set` field here, unconditionally,
    // before any read or write — the actual crash site (see README.md). `poll()` needs no
    // per-channel state to prime: build_and_send/send_to/next below construct a fresh
    // `libc::pollfd` on the stack each call, exactly like linux.rs already does. Nothing here
    // depends on the fd's numeric value anymore.
    tracing::debug!(fd = fd, interface = %network_interface.name, "opened BPF device");
    let fd = Arc::new(pnet_sys::FileDesc { fd: fd });
    let sender = Box::new(DataLinkSenderImpl {
        fd: fd.clone(),
        write_buffer: vec![0; config.write_buffer_size],
        loopback: loopback,
        timeout: config
            .write_timeout
            .map(|to| pnet_sys::duration_to_timespec(to)),
    });
    let receiver = Box::new(DataLinkReceiverImpl {
        fd: fd.clone(),
        read_buffer: vec![0; allocated_read_buffer_size],
        buffer_offset,
        loopback: loopback,
        timeout: config
            .read_timeout
            .map(|to| pnet_sys::duration_to_timespec(to)),
        // Enough room for minimally sized packets without reallocating
        packets: VecDeque::with_capacity(allocated_read_buffer_size / 64),
    });

    Ok(super::Channel::Ethernet(sender, receiver))
}

// SCANOPY LOCAL PATCH: `timeout_ms` conversion shared by both impls below, identical to the one
// linux.rs already uses (`poll`'s granularity is milliseconds, not the nanosecond `timespec`
// `pselect` took) — `-1` for block-forever.
fn timeout_ms(timeout: &Option<libc::timespec>) -> libc::c_int {
    timeout
        .as_ref()
        .map(|to| (to.tv_sec as i64 * 1000) + (to.tv_nsec as i64 / 1_000_000))
        .unwrap_or(-1) as libc::c_int
}

struct DataLinkSenderImpl {
    fd: Arc<pnet_sys::FileDesc>,
    write_buffer: Vec<u8>,
    loopback: bool,
    timeout: Option<libc::timespec>,
}

impl DataLinkSender for DataLinkSenderImpl {
    #[inline]
    fn build_and_send(
        &mut self,
        num_packets: usize,
        packet_size: usize,
        func: &mut dyn FnMut(&mut [u8]),
    ) -> Option<io::Result<()>> {
        let len = num_packets * packet_size;
        if len >= self.write_buffer.len() {
            None
        } else {
            // If we're sending on the loopback device, discard the ethernet header.
            // The OS will prepend the packet with 4 bytes set to AF_INET.
            let offset = if self.loopback {
                ETHERNET_HEADER_SIZE
            } else {
                0
            };
            for chunk in self.write_buffer[..len].chunks_mut(packet_size) {
                func(chunk);
                // SCANOPY LOCAL PATCH: poll(), not select()/FD_SET — see README.md. One pollfd
                // built fresh per call, exactly like linux.rs's DataLinkSenderImpl already does;
                // no per-channel fd_set state to keep in sync with a single monitored fd.
                //
                // Unlike linux.rs's AF_PACKET socket, this /dev/bpf character device does not
                // reliably set POLLOUT in `revents` even when the write below succeeds — bpf(4)
                // documents writes as unbuffered/synchronous and says nothing about poll/select
                // write-readiness. The original select()/FD_SET code this replaces never checked
                // FD_ISSET either; it trusted a positive return alone. Match that: any ret > 0
                // means proceed, regardless of which bits `revents` carries.
                let mut pollfd = libc::pollfd {
                    fd: self.fd.fd,
                    events: libc::POLLOUT,
                    revents: 0,
                };
                let ret = unsafe { libc::poll(&mut pollfd as *mut libc::pollfd, 1, timeout_ms(&self.timeout)) };
                if ret == -1 {
                    // Error occurred!
                    return Some(Err(io::Error::last_os_error()));
                } else if ret == 0 {
                    return Some(Err(io::Error::new(io::ErrorKind::TimedOut, "Timed out")));
                } else {
                    match unsafe {
                        libc::write(
                            self.fd.fd,
                            chunk.as_ptr().offset(offset as isize) as *const libc::c_void,
                            (chunk.len() - offset) as libc::size_t,
                        )
                    } {
                        len if len == -1 => return Some(Err(io::Error::last_os_error())),
                        _ => (),
                    }
                }
            }
            Some(Ok(()))
        }
    }

    #[inline]
    fn send_to(&mut self, packet: &[u8], _dst: Option<NetworkInterface>) -> Option<io::Result<()>> {
        // If we're sending on the loopback device, discard the ethernet header.
        // The OS will prepend the packet with 4 bytes set to AF_INET.
        let offset = if self.loopback {
            ETHERNET_HEADER_SIZE
        } else {
            0
        };
        // SCANOPY LOCAL PATCH: poll(), not select()/FD_SET — see build_and_send above (same
        // caveat: this BPF device's POLLOUT bit isn't trustworthy, so any ret > 0 is enough).
        let mut pollfd = libc::pollfd {
            fd: self.fd.fd,
            events: libc::POLLOUT,
            revents: 0,
        };
        let ret = unsafe { libc::poll(&mut pollfd as *mut libc::pollfd, 1, timeout_ms(&self.timeout)) };
        if ret == -1 {
            // Error occurred!
            return Some(Err(io::Error::last_os_error()));
        } else if ret == 0 {
            return Some(Err(io::Error::new(io::ErrorKind::TimedOut, "Timed out")));
        }
        match unsafe {
            libc::write(
                self.fd.fd,
                packet.as_ptr().offset(offset as isize) as *const libc::c_void,
                (packet.len() - offset) as libc::size_t,
            )
        } {
            len if len == -1 => Some(Err(io::Error::last_os_error())),
            _ => Some(Ok(())),
        }
    }
}

struct DataLinkReceiverImpl {
    fd: Arc<pnet_sys::FileDesc>,
    read_buffer: Vec<u8>,
    buffer_offset: usize,
    loopback: bool,
    timeout: Option<libc::timespec>,
    packets: VecDeque<(usize, usize)>,
}

impl DataLinkReceiver for DataLinkReceiverImpl {
    fn next(&mut self) -> io::Result<&[u8]> {
        let header_size = if self.loopback { NULL_HEADER_SIZE } else { 0 };

        if self.packets.is_empty() {
            let buffer = &mut self.read_buffer[self.buffer_offset..];
            // SCANOPY LOCAL PATCH: poll(), not select()/FD_SET — see README.md and
            // DataLinkSenderImpl::build_and_send above (same caveat: don't gate on which bits
            // `revents` carries, only on ret > 0, matching the original select()/FD_ISSET-free
            // code this replaces).
            let mut pollfd = libc::pollfd {
                fd: self.fd.fd,
                events: libc::POLLIN,
                revents: 0,
            };
            let ret = unsafe { libc::poll(&mut pollfd as *mut libc::pollfd, 1, timeout_ms(&self.timeout)) };
            if ret == -1 {
                return Err(io::Error::last_os_error());
            } else if ret == 0 {
                return Err(io::Error::new(io::ErrorKind::TimedOut, "Timed out"));
            } else {
                let buflen = match unsafe {
                    libc::read(
                        self.fd.fd,
                        buffer.as_ptr() as *mut libc::c_void,
                        buffer.len() as libc::size_t,
                    )
                } {
                    len if len > 0 => len,
                    _ => return Err(io::Error::last_os_error()),
                };
                let mut ptr = buffer.as_mut_ptr();
                let end = unsafe { buffer.as_ptr().offset(buflen as isize) };
                while (ptr as *const u8) < end {
                    unsafe {
                        let packet: *const bpf::bpf_hdr = mem::transmute(ptr);
                        let start =
                            ptr as isize + (*packet).bh_hdrlen as isize - buffer.as_ptr() as isize;
                        self.packets.push_back((
                            start as usize + header_size,
                            (*packet).bh_caplen as usize - header_size,
                        ));
                        let offset = (*packet).bh_hdrlen as isize + (*packet).bh_caplen as isize;
                        ptr = ptr.offset(bpf::BPF_WORDALIGN(offset));
                    }
                }
            }
        }
        let (start, mut len) = self.packets.pop_front().unwrap();
        len += self.buffer_offset;
        // Zero out part that will become fake ethernet header if on loopback.
        for i in (&mut self.read_buffer[start..start + self.buffer_offset]).iter_mut() {
            *i = 0;
        }
        Ok(&self.read_buffer[start..start + len])
    }
}

/// Get a list of available network interfaces for the current machine.
pub fn interfaces() -> Vec<NetworkInterface> {
    #[path = "unix_interfaces.rs"]
    mod interfaces;
    interfaces::interfaces()
}
