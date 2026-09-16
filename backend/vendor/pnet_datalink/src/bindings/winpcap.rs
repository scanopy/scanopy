// Copyright (c) 2014 Robert Clipsham <robert@octarineparrot.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

extern crate winapi;

use self::winapi::ctypes;
use self::winapi::shared::{guiddef, minwindef};
use self::winapi::um::{minwinbase, winnt};

use pnet_sys;

#[repr(C)]
pub struct _ADAPTER;
pub type ADAPTER = _ADAPTER;
pub type LPADAPTER = *mut _ADAPTER;

#[repr(C)]
pub struct _PACKET {
    pub hEvent: winnt::HANDLE,
    pub OverLapped: minwinbase::OVERLAPPED,
    pub Buffer: PVOID,
    pub Length: UINT,
    pub ulBytesReceived: minwindef::DWORD,
    pub bIoComplete: winnt::BOOLEAN,
}
pub type PACKET = _PACKET;
pub type LPPACKET = *mut _PACKET;

pub type TCHAR = ctypes::c_char;
pub type PTSTR = *mut TCHAR;

pub type PVOID = *mut ctypes::c_void;
pub type PCHAR = *mut winnt::CHAR;
pub type PWCHAR = *mut winnt::WCHAR;
pub type UINT = ctypes::c_uint;
pub type ULONG = ctypes::c_ulong;
pub type PULONG = *mut ULONG;
pub type ULONG64 = u64;
pub type UINT32 = u32;
pub type UINT8 = u8;
pub type INT = i32;

const MAX_ADAPTER_DESCRIPTION_LENGTH: usize = 128;
const MAX_ADAPTER_NAME_LENGTH: usize = 256;
const MAX_ADAPTER_ADDRESS_LENGTH: usize = 8;

// from ntddndis.h
pub const NDIS_PACKET_TYPE_PROMISCUOUS: ULONG = 0x00000020;

// from IPTypes.h
#[repr(C)]
pub struct _IP_ADDRESS_STRING {
    pub String: [ctypes::c_char; 4 * 4],
}

pub type IP_ADDRESS_STRING = _IP_ADDRESS_STRING;
pub type PIP_ADDRESS_STRING = *mut _IP_ADDRESS_STRING;
pub type IP_MASK_STRING = _IP_ADDRESS_STRING;
pub type PIP_MASK_STRING = *mut _IP_ADDRESS_STRING;

#[repr(C)]
pub struct _IP_ADDR_STRING {
    pub Next: *mut _IP_ADDR_STRING,
    pub IpAddress: IP_ADDRESS_STRING,
    pub IpMask: IP_MASK_STRING,
    pub Context: minwindef::DWORD,
}

pub type IP_ADDR_STRING = _IP_ADDR_STRING;
pub type PIP_ADDR_STRING = *mut _IP_ADDR_STRING;

#[repr(C)]
pub struct _IP_ADAPTER_INFO {
    pub Next: *mut _IP_ADAPTER_INFO,
    pub ComboIndex: minwindef::DWORD,
    pub AdapterName: [ctypes::c_char; MAX_ADAPTER_NAME_LENGTH + 4],
    pub Description: [ctypes::c_char; MAX_ADAPTER_DESCRIPTION_LENGTH + 4],
    pub AddressLength: UINT,
    pub Address: [minwindef::BYTE; MAX_ADAPTER_ADDRESS_LENGTH],
    pub Index: minwindef::DWORD,
    pub Type: UINT,
    pub DhcpEnabled: UINT,
    pub CurrentIpAddress: PIP_ADDR_STRING,
    pub IpAddressList: IP_ADDR_STRING,
    pub GatewayList: IP_ADDR_STRING,
    pub DhcpServer: IP_ADDR_STRING,
    pub HaveWins: minwindef::BOOL,
    pub PrimaryWinsServer: IP_ADDR_STRING,
    pub SecondaryWinsServer: IP_ADDR_STRING,
    pub LeaseObtained: libc::time_t,
    pub LeaseExpires: libc::time_t,
}

pub type IP_ADAPTER_INFO = _IP_ADAPTER_INFO;
pub type PIP_ADAPTER_INFO = *mut _IP_ADAPTER_INFO;

const MAX_DHCPV6_DUID_LENGTH: usize = 130;
const MAX_DNS_SUFFIX_STRING_LENGTH: usize = 256;

pub type LPSOCKADDR = *mut pnet_sys::SockAddr;

#[repr(C)]
pub struct _SOCKET_ADDRESS {
    pub lpSockaddr: LPSOCKADDR,
    pub iSockaddrLength: INT,
}

pub type SOCKET_ADDRESS = _SOCKET_ADDRESS;
pub type PSOCKET_ADDRESS = *mut _SOCKET_ADDRESS;

#[repr(C)]
pub enum IP_PREFIX_ORIGIN {
    IpPrefixOriginOther = 0,
    IpPrefixOriginManual,
    IpPrefixOriginWellKnown,
    IpPrefixOriginDhcp,
    IpPrefixOriginRouterAdvertisement,
    IpPrefixOriginUnchanged = 16,
}

#[repr(C)]
pub enum IP_SUFFIX_ORIGIN {
    IpSuffixOriginOther = 0,
    IpSuffixOriginManual,
    IpSuffixOriginWellKnown,
    IpSuffixOriginDhcp,
    IpSuffixOriginLinkLayerAddress,
    IpSuffixOriginRandom,
    IpSuffixOriginUnchanged = 16,
}

#[repr(C)]
pub enum IP_DAD_STATE {
    IpDadStateInvalid = 0,
    IpDadStateTentative,
    IpDadStateDuplicate,
    IpDadStateDeprecated,
    IpDadStatePreferred,
}

#[repr(C)]
pub enum IF_OPER_STATUS {
    IfOperStatusUp = 1,
    IfOperStatusDown,
    IfOperStatusTesting,
    IfOperStatusUnknown,
    IfOperStatusDormant,
    IfOperStatusNotPresent,
    IfOperStatusLowerLayerDown,
}

#[repr(C)]
pub struct _IP_ADAPTER_UNICAST_ADDRESS {
    pub Length: ULONG,
    pub Flags: minwindef::DWORD,
    pub Next: *mut _IP_ADAPTER_UNICAST_ADDRESS,
    pub Address: SOCKET_ADDRESS,
    pub PrefixOrigin: IP_PREFIX_ORIGIN,
    pub SuffixOrigin: IP_SUFFIX_ORIGIN,
    pub DadState: IP_DAD_STATE,
    pub ValidLifetime: ULONG,
    pub PreferredLifetime: ULONG,
    pub LeaseLifetime: ULONG,
    pub OnLinkPrefixLength: UINT8,
}

pub type IP_ADAPTER_UNICAST_ADDRESS = _IP_ADAPTER_UNICAST_ADDRESS;
pub type PIP_ADAPTER_UNICAST_ADDRESS = *mut _IP_ADAPTER_UNICAST_ADDRESS;

#[repr(C)]
pub struct _IP_ADAPTER_ANYCAST_ADDRESS {
    pub Length: ULONG,
    pub Flags: minwindef::DWORD,
    pub Next: *mut _IP_ADAPTER_ANYCAST_ADDRESS,
    pub Address: SOCKET_ADDRESS,
}

pub type IP_ADAPTER_ANYCAST_ADDRESS = _IP_ADAPTER_ANYCAST_ADDRESS;
pub type PIP_ADAPTER_ANYCAST_ADDRESS = *mut _IP_ADAPTER_ANYCAST_ADDRESS;

#[repr(C)]
pub struct _IP_ADAPTER_MULTICAST_ADDRESS {
    pub Length: ULONG,
    pub Flags: minwindef::DWORD,
    pub Next: *mut _IP_ADAPTER_MULTICAST_ADDRESS,
    pub Address: SOCKET_ADDRESS,
}

pub type IP_ADAPTER_MULTICAST_ADDRESS = _IP_ADAPTER_MULTICAST_ADDRESS;
pub type PIP_ADAPTER_MULTICAST_ADDRESS = *mut _IP_ADAPTER_MULTICAST_ADDRESS;

#[repr(C)]
pub struct _IP_ADAPTER_DNS_SERVER_ADDRESS {
    pub Length: ULONG,
    pub Flags: minwindef::DWORD,
    pub Next: *mut _IP_ADAPTER_DNS_SERVER_ADDRESS,
    pub Address: SOCKET_ADDRESS,
}

pub type IP_ADAPTER_DNS_SERVER_ADDRESS = _IP_ADAPTER_DNS_SERVER_ADDRESS;
pub type PIP_ADAPTER_DNS_SERVER_ADDRESS = *mut _IP_ADAPTER_DNS_SERVER_ADDRESS;

#[repr(C)]
pub struct _IP_ADAPTER_PREFIX {
    pub Length: ULONG,
    pub Flags: minwindef::DWORD,
    pub Next: *mut _IP_ADAPTER_PREFIX,
    pub Address: SOCKET_ADDRESS,
    pub PrefixLength: ULONG,
}

pub type IP_ADAPTER_PREFIX = _IP_ADAPTER_PREFIX;
pub type PIP_ADAPTER_PREFIX = *mut _IP_ADAPTER_PREFIX;

#[repr(C)]
pub struct _IP_ADAPTER_WINS_SERVER_ADDRESS_LH {
    pub Length: ULONG,
    pub Reserved: minwindef::DWORD,
    pub Next: *mut _IP_ADAPTER_WINS_SERVER_ADDRESS_LH,
    pub Address: SOCKET_ADDRESS,
}

pub type IP_ADAPTER_WINS_SERVER_ADDRESS_LH = _IP_ADAPTER_WINS_SERVER_ADDRESS_LH;
pub type PIP_ADAPTER_WINS_SERVER_ADDRESS_LH = *mut _IP_ADAPTER_WINS_SERVER_ADDRESS_LH;
pub type IP_ADAPTER_WINS_SERVER_ADDRESS = _IP_ADAPTER_WINS_SERVER_ADDRESS_LH;
pub type PIP_ADAPTER_WINS_SERVER_ADDRESS = *mut _IP_ADAPTER_WINS_SERVER_ADDRESS_LH;

#[repr(C)]
pub struct _IP_ADAPTER_GATEWAY_ADDRESS_LH {
    pub Length: ULONG,
    pub Reserved: minwindef::DWORD,
    pub Next: *mut _IP_ADAPTER_GATEWAY_ADDRESS_LH,
    pub Address: SOCKET_ADDRESS,
}

pub type IP_ADAPTER_GATEWAY_ADDRESS_LH = _IP_ADAPTER_GATEWAY_ADDRESS_LH;
pub type PIP_ADAPTER_GATEWAY_ADDRESS_LH = *mut _IP_ADAPTER_GATEWAY_ADDRESS_LH;
pub type IP_ADAPTER_GATEWAY_ADDRESS = _IP_ADAPTER_GATEWAY_ADDRESS_LH;
pub type PIP_ADAPTER_GATEWAY_ADDRESS = *mut _IP_ADAPTER_GATEWAY_ADDRESS_LH;

pub type NET_IF_COMPARTMENT_ID = UINT32;
pub type PNET_IF_COMPARTMENT_ID = *mut UINT32;
pub type NET_IF_NETWORK_GUID = guiddef::GUID;
pub type PNET_IF_NETWORK_GUID = *mut guiddef::GUID;

#[repr(C)]
pub enum _NET_IF_CONNECTION_TYPE {
    NET_IF_CONNECTION_DEDICATED = 1,
    NET_IF_CONNECTION_PASSIVE = 2,
    NET_IF_CONNECTION_DEMAND = 3,
    NET_IF_CONNECTION_MAXIMUM = 4,
}

pub type NET_IF_CONNECTION_TYPE = _NET_IF_CONNECTION_TYPE;
pub type PNET_IF_CONNECTION_TYPE = *mut _NET_IF_CONNECTION_TYPE;

#[repr(C)]
pub struct _NET_LUID_LH {
    pub Value: ULONG64,
}
pub type NET_LUID_LH = _NET_LUID_LH;
pub type PNER_LUID_LH = *mut _NET_LUID_LH;
pub type NET_LUID = NET_LUID_LH;
pub type PNET_LUID = *mut NET_LUID;
pub type IF_LUID = NET_LUID;
pub type PIF_LUID = *mut NET_LUID;

#[repr(C)]
pub enum TUNNEL_TYPE {
    TUNNEL_TYPE_NONE = 0,
    TUNNEL_TYPE_OTHER = 1,
    TUNNEL_TYPE_DIRECT = 2,
    TUNNEL_TYPE_6TO4 = 11,
    TUNNEL_TYPE_ISATAP = 13,
    TUNNEL_TYPE_TEREDO = 14,
    TUNNEL_TYPE_IPHTTPS = 15,
}

pub type PTUNNEL_TYPE = *mut TUNNEL_TYPE;

#[repr(C)]
pub struct _IP_ADAPTER_DNS_SUFFIX {
    pub Next: *mut _IP_ADAPTER_DNS_SUFFIX,
    pub String: [winnt::WCHAR; MAX_DNS_SUFFIX_STRING_LENGTH],
}

pub type IP_ADAPTER_DNS_SUFFIX = _IP_ADAPTER_DNS_SUFFIX;
pub type PIP_ADAPTER_DNS_SUFFIX = *mut _IP_ADAPTER_DNS_SUFFIX;

#[repr(C)]
pub struct _IP_ADAPTER_ADDRESSES {
    pub Length: ULONG,
    pub IfIndex: minwindef::DWORD,
    pub Next: *mut _IP_ADAPTER_ADDRESSES,
    pub AdapterName: PCHAR,
    pub FirstUnicastAddress: PIP_ADAPTER_UNICAST_ADDRESS,
    pub FirstAnycastAddress: PIP_ADAPTER_ANYCAST_ADDRESS,
    pub FirstMulticastAddress: PIP_ADAPTER_MULTICAST_ADDRESS,
    pub FirstDnsServerAddress: PIP_ADAPTER_DNS_SERVER_ADDRESS,
    pub DnsSuffix: PWCHAR,
    pub Description: PWCHAR,
    pub FriendlyName: PWCHAR,
    pub PhysicalAddress: [minwindef::BYTE; MAX_ADAPTER_ADDRESS_LENGTH],
    pub PhysicalAddressLength: minwindef::DWORD,
    pub Flags: minwindef::DWORD,
    pub Mtu: minwindef::DWORD,
    pub IfType: minwindef::DWORD,
    pub OperStatus: IF_OPER_STATUS,
    pub Ipv6IfIndex: minwindef::DWORD,
    pub ZoneIndices: [minwindef::DWORD; 16],
    pub FirstPrefix: PIP_ADAPTER_PREFIX,
    pub TransmitLinkSpeed: ULONG64,
    pub ReceiveLinkSpeed: ULONG64,
    pub FirstWinsServerAddress: PIP_ADAPTER_WINS_SERVER_ADDRESS_LH,
    pub FirstGatewayAddress: PIP_ADAPTER_GATEWAY_ADDRESS_LH,
    pub Ipv4Metric: ULONG,
    pub Ipv6Metric: ULONG,
    pub Luid: IF_LUID,
    pub Dhcpv4Server: SOCKET_ADDRESS,
    pub CompartmentId: NET_IF_COMPARTMENT_ID,
    pub NetworkGuid: NET_IF_NETWORK_GUID,
    pub ConnectionType: NET_IF_CONNECTION_TYPE,
    pub TunnelType: TUNNEL_TYPE,
    pub Dhcpv6Server: SOCKET_ADDRESS,
    pub Dhcpv6ClientDuid: [minwindef::BYTE; MAX_DHCPV6_DUID_LENGTH],
    pub Dhcpv6ClientDuidLength: ULONG,
    pub Dhcpv6Iaid: ULONG,
    pub FirstDnsSuffix: PIP_ADAPTER_DNS_SUFFIX,
}

pub type IP_ADAPTER_ADDRESSES = _IP_ADAPTER_ADDRESSES;
pub type PIP_ADAPTER_ADDRESSES = *mut _IP_ADAPTER_ADDRESSES;

#[link(name = "iphlpapi")]
extern "system" {

    // from IPHlpApi.h
    pub fn GetAdaptersInfo(pAdapterInfo: PIP_ADAPTER_INFO, pOutBufLen: PULONG) -> minwindef::DWORD;
    pub fn GetAdaptersAddresses(
        Family: ULONG,
        Flags: ULONG,
        Reserved: PVOID,
        AdapterAddresses: PIP_ADAPTER_ADDRESSES,
        SizePointer: PULONG,
    ) -> minwindef::DWORD;
}

#[link(name = "Packet")]
#[allow(improper_ctypes)]
extern "C" {
    // from Packet32.h
    pub fn PacketSendPacket(
        AdapterObject: LPADAPTER,
        pPacket: LPPACKET,
        Sync: winnt::BOOLEAN,
    ) -> winnt::BOOLEAN;
    pub fn PacketReceivePacket(
        AdapterObject: LPADAPTER,
        lpPacket: LPPACKET,
        Sync: winnt::BOOLEAN,
    ) -> winnt::BOOLEAN;
    pub fn PacketAllocatePacket() -> LPPACKET;
    pub fn PacketInitPacket(lpPacket: LPPACKET, Buffer: PVOID, Length: UINT);
    pub fn PacketFreePacket(lpPacket: LPPACKET);
    pub fn PacketOpenAdapter(AdapterName: PCHAR) -> LPADAPTER;
    pub fn PacketCloseAdapter(lpAdapter: LPADAPTER);
    pub fn PacketGetAdapterNames(pStr: PTSTR, BufferSize: PULONG) -> winnt::BOOLEAN;
    pub fn PacketSetHwFilter(AdapterObject: LPADAPTER, Filter: ULONG) -> winnt::BOOLEAN;
    pub fn PacketSetMinToCopy(AdapterObject: LPADAPTER, nbytes: ctypes::c_int) -> winnt::BOOLEAN;
    pub fn PacketSetBuff(AdapterObject: LPADAPTER, dim: ctypes::c_int) -> winnt::BOOLEAN;
    pub fn PacketSetReadTimeout(AdapterObject: LPADAPTER, timeout: ctypes::c_int)
        -> winnt::BOOLEAN;
}
