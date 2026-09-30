// Copyright (c) 2014-2016 Robert Clipsham <robert@octarineparrot.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Support for sending and receiving data link layer packets using the WinPcap library.

use super::bindings::{bpf, winpcap};
use super::{DataLinkReceiver, DataLinkSender, MacAddr, NetworkInterface};

use ipnetwork::{ip_mask_to_prefix, IpNetwork};

use std::cmp;
use std::collections::VecDeque;
use std::ffi::{CStr, CString};
use std::io;
use std::mem;
use std::slice;
use std::str::from_utf8_unchecked;
use std::sync::Arc;
use winapi::ctypes::c_char;

use winapi::ctypes;

struct WinPcapAdapter {
    adapter: winpcap::LPADAPTER,
}

impl Drop for WinPcapAdapter {
    fn drop(&mut self) {
        unsafe {
            winpcap::PacketCloseAdapter(self.adapter);
        }
    }
}

struct WinPcapPacket {
    packet: winpcap::LPPACKET,
}

impl Drop for WinPcapPacket {
    fn drop(&mut self) {
        unsafe {
            winpcap::PacketFreePacket(self.packet);
        }
    }
}

/// The WinPcap's specific configuration.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Config {
    /// The size of buffer to use when writing packets. Defaults to 4096.
    pub write_buffer_size: usize,

    /// The size of buffer to use when reading packets. Defaults to 4096.
    pub read_buffer_size: usize,
}

impl<'a> From<&'a super::Config> for Config {
    fn from(config: &super::Config) -> Config {
        Config {
            write_buffer_size: config.write_buffer_size,
            read_buffer_size: config.read_buffer_size,
        }
    }
}

impl Default for Config {
    fn default() -> Config {
        Config {
            write_buffer_size: 4096,
            read_buffer_size: 4096,
        }
    }
}

/// Create a datalink channel using the WinPcap library.
#[inline]
pub fn channel(network_interface: &NetworkInterface, config: Config) -> io::Result<super::Channel> {
    // SCANOPY LOCAL PATCH: every Packet* call below goes through the delay-loaded packet.dll. With
    // no packet.dll the loader raises 0xC06D007E and the process dies, so refuse here instead.
    if !packet_dll_available() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Npcap is not installed (packet.dll could not be loaded)",
        ));
    }

    let mut read_buffer = Vec::new();
    read_buffer.resize(config.read_buffer_size, 0u8);

    let mut write_buffer = Vec::new();
    write_buffer.resize(config.write_buffer_size, 0u8);

    let adapter = unsafe {
        let net_if_str = CString::new(network_interface.name.as_bytes()).unwrap();
        winpcap::PacketOpenAdapter(net_if_str.as_ptr() as *mut ctypes::c_char)
    };
    if adapter.is_null() {
        return Err(io::Error::last_os_error());
    }

    let ret = unsafe { winpcap::PacketSetHwFilter(adapter, winpcap::NDIS_PACKET_TYPE_PROMISCUOUS) };
    if ret == 0 {
        return Err(io::Error::last_os_error());
    }

    // Set kernel buffer size
    let ret = unsafe { winpcap::PacketSetBuff(adapter, config.read_buffer_size as ctypes::c_int) };
    if ret == 0 {
        return Err(io::Error::last_os_error());
    }

    // Immediate mode
    let ret = unsafe { winpcap::PacketSetMinToCopy(adapter, 1) };
    if ret == 0 {
        return Err(io::Error::last_os_error());
    }

    let read_packet = unsafe { winpcap::PacketAllocatePacket() };
    if read_packet.is_null() {
        unsafe {
            winpcap::PacketCloseAdapter(adapter);
        }
        return Err(io::Error::last_os_error());
    }

    unsafe {
        winpcap::PacketInitPacket(
            read_packet,
            read_buffer.as_mut_ptr() as winpcap::PVOID,
            config.read_buffer_size as winpcap::UINT,
        )
    }

    let write_packet = unsafe { winpcap::PacketAllocatePacket() };
    if write_packet.is_null() {
        unsafe {
            winpcap::PacketFreePacket(read_packet);
            winpcap::PacketCloseAdapter(adapter);
        }
        return Err(io::Error::last_os_error());
    }

    unsafe {
        winpcap::PacketInitPacket(
            write_packet,
            write_buffer.as_mut_ptr() as winpcap::PVOID,
            config.write_buffer_size as winpcap::UINT,
        )
    }

    let adapter = Arc::new(WinPcapAdapter { adapter: adapter });
    let sender = Box::new(DataLinkSenderImpl {
        adapter: adapter.clone(),
        _write_buffer: write_buffer,
        packet: WinPcapPacket {
            packet: write_packet,
        },
    });
    let receiver = Box::new(DataLinkReceiverImpl {
        adapter: adapter,
        _read_buffer: read_buffer,
        packet: WinPcapPacket {
            packet: read_packet,
        },
        // Enough room for minimally sized packets without reallocating
        packets: VecDeque::with_capacity(unsafe { (*read_packet).Length } as usize / 64),
    });
    Ok(super::Channel::Ethernet(sender, receiver))
}

struct DataLinkSenderImpl {
    adapter: Arc<WinPcapAdapter>,
    _write_buffer: Vec<u8>,
    packet: WinPcapPacket,
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
        if len >= unsafe { (*self.packet.packet).Length } as usize {
            None
        } else {
            let min = unsafe { cmp::min((*self.packet.packet).Length as usize, len) };
            let slice: &mut [u8] =
                unsafe { slice::from_raw_parts_mut((*self.packet.packet).Buffer as *mut u8, min) };
            for chunk in slice.chunks_mut(packet_size) {
                func(chunk);

                // Make sure the right length of packet is sent
                let old_len = unsafe { (*self.packet.packet).Length };
                unsafe {
                    (*self.packet.packet).Length = packet_size as u32;
                }

                let ret = unsafe {
                    winpcap::PacketSendPacket(self.adapter.adapter, self.packet.packet, 0)
                };

                unsafe {
                    (*self.packet.packet).Length = old_len;
                }

                if ret == 0 {
                    return Some(Err(io::Error::last_os_error()));
                }
            }
            Some(Ok(()))
        }
    }

    #[inline]
    fn send_to(&mut self, packet: &[u8], _dst: Option<NetworkInterface>) -> Option<io::Result<()>> {
        self.build_and_send(1, packet.len(), &mut |eh: &mut [u8]| {
            eh.copy_from_slice(packet);
        })
    }
}

unsafe impl Send for DataLinkSenderImpl {}
unsafe impl Sync for DataLinkSenderImpl {}

struct DataLinkReceiverImpl {
    adapter: Arc<WinPcapAdapter>,
    _read_buffer: Vec<u8>,
    packet: WinPcapPacket,
    packets: VecDeque<(usize, usize)>,
}

unsafe impl Send for DataLinkReceiverImpl {}
unsafe impl Sync for DataLinkReceiverImpl {}

impl DataLinkReceiver for DataLinkReceiverImpl {
    fn next(&mut self) -> io::Result<&[u8]> {
        // NOTE Most of the logic here is identical to FreeBSD/OS X
        while self.packets.is_empty() {
            let ret = unsafe {
                winpcap::PacketReceivePacket(self.adapter.adapter, self.packet.packet, 0)
            };
            let buflen = match ret {
                0 => return Err(io::Error::last_os_error()),
                _ => unsafe { (*self.packet.packet).ulBytesReceived as isize },
            };
            let mut ptr = unsafe { (*self.packet.packet).Buffer  as *mut c_char};
            let end = unsafe { ((*self.packet.packet).Buffer as *mut c_char).offset(buflen) };
            while ptr < end {
                unsafe {
                    let packet: *const bpf::bpf_hdr = mem::transmute(ptr);
                    let start = ptr as isize + (*packet).bh_hdrlen as isize
                        - (*self.packet.packet).Buffer as isize;
                    self.packets
                        .push_back((start as usize, (*packet).bh_caplen as usize));
                    let offset = (*packet).bh_hdrlen as isize + (*packet).bh_caplen as isize;
                    ptr = ptr.offset(bpf::BPF_WORDALIGN(offset));
                }
            }
        }
        let (start, len) = self.packets.pop_front().unwrap();
        let slice = unsafe {
            let data = (*self.packet.packet).Buffer as usize + start;
            slice::from_raw_parts(data as *const u8, len)
        };
        Ok(slice)
    }
}

/// Whether packet.dll can be loaded, so that a Packet* call won't fail its delay-load.
///
/// SCANOPY LOCAL PATCH. The daemon links packet.dll with `/DELAYLOAD`, so a missing DLL
/// surfaces at the first Packet* call as an SEH exception (0xC06D007E) that kills the process.
/// Npcap's default install puts packet.dll only in `System32\Npcap`, which is not on the DLL
/// search path (only its WinPcap-compatible mode also copies it into `System32`). Loading it by
/// full path from there first puts it in the process's module list, and the delay-load helper's
/// later `LoadLibrary("packet.dll")` resolves to the already-loaded module by name. Then fall
/// back to the normal search path for WinPcap-compatible installs. Checked once per process.
pub fn packet_dll_available() -> bool {
    static AVAILABLE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        let load = |path: &str, flags: u32| -> bool {
            let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
            !unsafe { winpcap::LoadLibraryExW(wide.as_ptr(), std::ptr::null_mut(), flags) }
                .is_null()
        };

        let mut sys_dir = [0u16; 260];
        let len = unsafe { winpcap::GetSystemDirectoryW(sys_dir.as_mut_ptr(), sys_dir.len() as u32) }
            as usize;
        if len > 0 && len < sys_dir.len() {
            let npcap = format!(
                "{}\\Npcap\\Packet.dll",
                String::from_utf16_lossy(&sys_dir[..len])
            );
            if load(&npcap, winpcap::LOAD_WITH_ALTERED_SEARCH_PATH) {
                return true;
            }
        }
        load("packet.dll", 0)
    })
}

/// Get a list of available network interfaces for the current machine.
///
/// SCANOPY LOCAL PATCH: upstream intersected this list with `PacketGetAdapterNames`, a packet.dll
/// call, so merely listing NICs crashed the process on a host without Npcap (see
/// `packet_dll_available`). Npcap names each adapter `\Device\NPF_{<AdapterName>}`, the same GUID
/// `GetAdaptersInfo` reports, so build that name directly and the result, names included, is what
/// upstream returned whenever Npcap is installed. Both `GetAdaptersInfo` return codes are also
/// checked now: upstream walked the buffer even when the second call failed, reading
/// uninitialised memory whenever the adapter list grew between the two calls.
pub fn interfaces() -> Vec<NetworkInterface> {
    const ERROR_SUCCESS: u32 = 0;
    const ERROR_BUFFER_OVERFLOW: u32 = 111;
    const ATTEMPTS: usize = 3;

    let entry_size = mem::size_of::<winpcap::IP_ADAPTER_INFO>() as u32;
    let mut adapters_size = 0u32;
    let mut adapters: Vec<winpcap::IP_ADAPTER_INFO> = Vec::new();
    let mut filled = false;

    // FIXME [windows] This only gets IPv4 addresses - should use GetAdaptersAddresses
    for _ in 0..ATTEMPTS {
        let vec_size = adapters_size.div_ceil(entry_size).max(1) as usize;
        adapters = Vec::with_capacity(vec_size);
        let mut buf_size = (vec_size as u32) * entry_size;
        let ret = unsafe {
            std::ptr::write_bytes(adapters.as_mut_ptr(), 0, vec_size);
            winpcap::GetAdaptersInfo(adapters.as_mut_ptr(), &mut buf_size)
        };
        match ret {
            ERROR_SUCCESS => {
                filled = true;
                break;
            }
            // The list grew since the size was read; retry with the new size.
            ERROR_BUFFER_OVERFLOW => adapters_size = buf_size,
            // ERROR_NO_DATA (no adapters) or a real failure: nothing to walk.
            _ => return Vec::new(),
        }
    }
    if !filled {
        return Vec::new();
    }

    // Create a complete list of NetworkInterfaces for the machine
    let mut cursor = adapters.as_mut_ptr();
    let mut all_ifaces = Vec::with_capacity(adapters.capacity());
    while !cursor.is_null() {
        let mac = unsafe {
            MacAddr(
                (*cursor).Address[0],
                (*cursor).Address[1],
                (*cursor).Address[2],
                (*cursor).Address[3],
                (*cursor).Address[4],
                (*cursor).Address[5],
            )
        };
        let mut ip_cursor = unsafe { &mut (*cursor).IpAddressList as winpcap::PIP_ADDR_STRING };
        let mut ips = Vec::new();
        while !ip_cursor.is_null() {
            if let Ok(ip_network) = parse_ip_network(ip_cursor) {
                ips.push(ip_network);
            }
            ip_cursor = unsafe { (*ip_cursor).Next };
        }

        unsafe {
            let name_str_ptr = (*cursor).AdapterName.as_ptr() as *const i8;

            let bytes = CStr::from_ptr(name_str_ptr).to_bytes();
            let name_str = from_utf8_unchecked(bytes).to_owned();

            let description_str_ptr = (*cursor).Description.as_ptr() as *const i8;
            let bytes = CStr::from_ptr(description_str_ptr).to_bytes();
            let description_str = from_utf8_unchecked(bytes).to_owned();

            all_ifaces.push(NetworkInterface {
                name: format!("\\Device\\NPF_{}", name_str),
                description: description_str,
                index: (*cursor).Index,
                mac: Some(mac),
                ips: ips,
                // flags: (*cursor).Type, // FIXME [windows]
                flags: 0,
            });

            cursor = (*cursor).Next;
        }
    }

    all_ifaces
}

fn parse_ip_network(ip_cursor: winpcap::PIP_ADDR_STRING) -> Result<IpNetwork, ()> {
    let ip_str_ptr = unsafe { &(*ip_cursor) }.IpAddress.String.as_ptr() as *const i8;
    let ip_bytes = unsafe { CStr::from_ptr(ip_str_ptr).to_bytes() };
    let ip_str = unsafe { from_utf8_unchecked(ip_bytes).to_owned() };
    let ip = ip_str.parse().map_err(|_| ())?;

    let mask_str_ptr = unsafe { &(*ip_cursor) }.IpMask.String.as_ptr() as *const i8;
    let mask_bytes = unsafe { CStr::from_ptr(mask_str_ptr).to_bytes() };
    let mask_str = unsafe { from_utf8_unchecked(mask_bytes).to_owned() };
    let mask = mask_str.parse().map_err(|_| ())?;

    let prefix = ip_mask_to_prefix(mask).map_err(|_| ())?;
    IpNetwork::new(ip, prefix).map_err(|_| ())
}
