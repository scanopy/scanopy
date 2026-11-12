#[cfg(feature = "v3")]
use crate::v3;
use crate::{
    BUFFER_SIZE, Error, MessageType, Oid, Result, Value, Varbinds, Version,
    asn1::{self, AsnReader},
    snmp,
};
use std::{
    fmt, mem,
    net::{IpAddr, Ipv4Addr},
    ops, ptr,
};

pub(crate) struct Buf {
    len: usize,
    #[cfg(not(feature = "heap_buffers"))]
    buf: [u8; BUFFER_SIZE],

    #[cfg(feature = "heap_buffers")]
    buf: Box<[u8]>,
}

impl fmt::Debug for Buf {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_list().entries(&self[..]).finish()
    }
}

impl Default for Buf {
    #[allow(clippy::large_stack_arrays)]
    fn default() -> Buf {
        Buf {
            len: 0,
            #[cfg(not(feature = "heap_buffers"))]
            buf: [0; BUFFER_SIZE],
            #[cfg(feature = "heap_buffers")]
            buf: vec![0; BUFFER_SIZE].into_boxed_slice(),
        }
    }
}

impl ops::Deref for Buf {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.buf[BUFFER_SIZE - self.len..]
    }
}

impl ops::DerefMut for Buf {
    fn deref_mut(&mut self) -> &mut [u8] {
        &mut self.buf[BUFFER_SIZE - self.len..]
    }
}

impl Buf {
    pub fn available(&mut self) -> &mut [u8] {
        &mut self.buf[..(BUFFER_SIZE - self.len)]
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.len
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn push_chunk(&mut self, chunk: &[u8]) {
        let offset = BUFFER_SIZE - self.len;
        self.buf[(offset - chunk.len())..offset].copy_from_slice(chunk);
        self.len += chunk.len();
    }

    pub fn push_byte(&mut self, byte: u8) {
        self.buf[BUFFER_SIZE - self.len - 1] = byte;
        self.len += 1;
    }

    pub fn reset(&mut self) {
        self.len = 0;
    }

    pub fn scribble_bytes<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut [u8]) -> usize,
    {
        let scribbled = f(self.available());
        self.len += scribbled;
    }

    pub fn push_constructed<F>(&mut self, ident: u8, mut f: F)
    where
        F: FnMut(&mut Self),
    {
        let before_len = self.len;
        f(self);
        let written = self.len - before_len;
        self.push_length(written);
        self.push_byte(ident);
    }

    pub fn push_sequence<F>(&mut self, f: F)
    where
        F: FnMut(&mut Self),
    {
        self.push_constructed(asn1::TYPE_SEQUENCE, f);
    }

    // fn push_set<F>(&mut self, f: F)
    //     where F: FnMut(&mut Self)
    // {
    //     self.push_constructed(asn1::TYPE_SET, f)
    // }

    #[allow(clippy::cast_possible_truncation)]
    pub fn push_length(&mut self, len: usize) {
        if len < 128 {
            // short form
            self.push_byte(len as u8);
        } else {
            // long form
            let num_leading_nulls = (len.leading_zeros() / 8) as usize;
            let length_len = mem::size_of::<usize>() - num_leading_nulls;
            let leading_byte = length_len as u8 | 0b1000_0000;
            self.scribble_bytes(|o| {
                if o.len() <= length_len {
                    return 0;
                }
                let bytes = len.to_be_bytes();
                let write_offset = o.len() - length_len - 1;
                o[write_offset] = leading_byte;
                o[write_offset + 1..].copy_from_slice(&bytes[num_leading_nulls..]);
                length_len + 1
            });
        }
    }

    pub fn push_integer(&mut self, n: i64) {
        let len = self.push_i64(n);
        self.push_length(len);
        self.push_byte(asn1::TYPE_INTEGER);
    }

    pub fn push_endofmibview(&mut self) {
        self.push_chunk(&[snmp::SNMP_ENDOFMIBVIEW, 0]);
    }

    pub fn push_nosuchobject(&mut self) {
        self.push_chunk(&[snmp::SNMP_NOSUCHOBJECT, 0]);
    }

    pub fn push_nosuchinstance(&mut self) {
        self.push_chunk(&[snmp::SNMP_NOSUCHINSTANCE, 0]);
    }

    pub fn push_counter32(&mut self, n: u32) {
        let len = self.push_i64(i64::from(n));
        self.push_length(len);
        self.push_byte(snmp::TYPE_COUNTER32);
    }

    pub fn push_unsigned32(&mut self, n: u32) {
        let len = self.push_i64(i64::from(n));
        self.push_length(len);
        self.push_byte(snmp::TYPE_UNSIGNED32);
    }

    pub fn push_timeticks(&mut self, n: u32) {
        let len = self.push_i64(i64::from(n));
        self.push_length(len);
        self.push_byte(snmp::TYPE_TIMETICKS);
    }

    pub fn push_opaque(&mut self, bytes: &[u8]) {
        self.push_chunk(bytes);
        self.push_length(bytes.len());
        self.push_byte(snmp::TYPE_OPAQUE);
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    pub fn push_counter64(&mut self, n: u64) {
        let len = self.push_i64(n as i64);
        self.push_length(len);
        self.push_byte(snmp::TYPE_COUNTER64);
    }

    pub fn push_i64(&mut self, mut n: i64) -> usize {
        let (null, num_null_bytes) = if n.is_negative() {
            (0xffu8, ((!n).leading_zeros() / 8) as usize)
        } else {
            (0x00u8, (n.leading_zeros() / 8) as usize)
        };
        n = n.to_be();
        let count = unsafe {
            let wbuf = self.available();
            let mut src_ptr = ptr::addr_of!(n).cast::<u8>();
            let mut dst_ptr = wbuf.as_mut_ptr().add(wbuf.len() - mem::size_of::<i64>());
            let mut count = mem::size_of::<i64>() - num_null_bytes;
            if count == 0 {
                count = 1;
            }
            // preserve sign
            if (*src_ptr.add(mem::size_of::<i64>() - count) ^ null) > 127u8 {
                count += 1;
            }
            if wbuf.len() < count {
                return 0;
            }
            #[allow(clippy::cast_possible_wrap)]
            let offset = (mem::size_of::<i64>() - count) as isize;
            src_ptr = src_ptr.offset(offset);
            dst_ptr = dst_ptr.offset(offset);
            ptr::copy_nonoverlapping(src_ptr, dst_ptr, count);
            count
        };
        self.len += count;
        count
    }

    pub fn push_boolean(&mut self, boolean: bool) {
        if boolean {
            self.push_byte(0x1);
        } else {
            self.push_byte(0x0);
        }
        self.push_length(1);
        self.push_byte(asn1::TYPE_BOOLEAN);
    }

    pub fn push_ipaddress(&mut self, ip: [u8; 4]) {
        self.push_chunk(&ip);
        self.push_length(ip.len());
        self.push_byte(snmp::TYPE_IPADDRESS);
    }

    pub fn push_null(&mut self) {
        self.push_chunk(&[asn1::TYPE_NULL, 0]);
    }

    pub fn push_object_identifier_raw(&mut self, input: &[u8]) {
        self.push_chunk(input);
        self.push_length(input.len());
        self.push_byte(asn1::TYPE_OBJECTIDENTIFIER);
    }

    #[allow(dead_code, clippy::cast_possible_truncation)]
    pub fn push_object_identifier(&mut self, input: &[u32]) {
        if input.len() < 2 {
            return;
        }
        let length_before = self.len;

        self.scribble_bytes(|output| {
            let mut pos = output.len() - 1;
            let (head, tail) = input.split_at(2);
            if head[0] >= 3 || head[1] >= 40 {
                return 0;
            }

            // encode the subids in reverse order
            for subid in tail.iter().rev() {
                let mut subid = *subid;
                let mut last_byte = true;
                loop {
                    if pos == 0 {
                        return 0;
                    }
                    if last_byte {
                        // continue bit is cleared
                        output[pos] = (subid & 0b0111_1111) as u8;
                        last_byte = false;
                    } else {
                        // continue bit is set
                        output[pos] = (subid | 0b1000_0000) as u8;
                    }
                    pos -= 1;
                    subid >>= 7;

                    if subid == 0 {
                        break;
                    }
                }
            }

            // encode the head last
            output[pos] = (head[0] * 40 + head[1]) as u8;
            output.len() - pos
        });
        let length_after = self.len;
        self.push_length(length_after - length_before);
        self.push_byte(asn1::TYPE_OBJECTIDENTIFIER);
    }

    pub fn push_octet_string(&mut self, bytes: &[u8]) {
        self.push_chunk(bytes);
        self.push_length(bytes.len());
        self.push_byte(asn1::TYPE_OCTETSTRING);
    }

    /// Convert buffer to Vec<u8> for UDP transmission
    pub fn to_vec(&self) -> Vec<u8> {
        (**self).to_vec()
    }
}

/// For reply: non_repeaters = error_status, max_repetitions = error_index
#[allow(clippy::too_many_arguments, clippy::unnecessary_wraps)]
#[inline]
pub(crate) fn build(
    version: Version,
    community: &[u8],
    ident: u8,
    req_id: i32,
    values: &[(&Oid, Value)],
    non_repeaters: u32,
    max_repetitions: u32,
    buf: &mut Buf,
    #[cfg(feature = "v3")] security: Option<&v3::Security>,
) -> Result<()> {
    #[cfg(feature = "v3")]
    if version == Version::V3 {
        return v3::build(
            ident,
            req_id,
            values,
            non_repeaters,
            max_repetitions,
            buf,
            security,
        );
    }
    buf.reset();
    buf.push_sequence(|buf| {
        build_inner(req_id, ident, values, max_repetitions, non_repeaters, buf);
        buf.push_octet_string(community);
        buf.push_integer(version as i64);
    });
    Ok(())
}

pub(crate) fn push_varbinds(buf: &mut Buf, values: &[(&Oid, Value)]) {
    buf.push_sequence(|buf| {
        for &(oid, ref val) in values.iter().rev() {
            buf.push_sequence(|buf| {
                match *val {
                    Value::Boolean(b) => buf.push_boolean(b),
                    Value::Null => buf.push_null(),
                    Value::Integer(i) => buf.push_integer(i),
                    Value::OctetString(ostr) => buf.push_octet_string(ostr),
                    Value::ObjectIdentifier(ref objid) => {
                        buf.push_object_identifier_raw(objid.as_bytes());
                    }
                    Value::IpAddress(ip) => buf.push_ipaddress(ip),
                    Value::Counter32(i) => buf.push_counter32(i),
                    Value::Unsigned32(i) => buf.push_unsigned32(i),
                    Value::Timeticks(tt) => buf.push_timeticks(tt),
                    Value::Opaque(bytes) => buf.push_opaque(bytes),
                    Value::Counter64(i) => buf.push_counter64(i),
                    Value::EndOfMibView => buf.push_endofmibview(),
                    Value::NoSuchObject => buf.push_nosuchobject(),
                    Value::NoSuchInstance => buf.push_nosuchinstance(),
                    _ => return,
                }
                buf.push_object_identifier_raw(oid.as_bytes());
            });
        }
    });
}

#[inline]
pub(crate) fn build_inner(
    req_id: i32,
    ident: u8,
    values: &[(&Oid, Value)],
    non_repeaters: u32,
    max_repetitions: u32,
    buf: &mut Buf,
) {
    buf.push_constructed(ident, |buf| {
        push_varbinds(buf, values);
        buf.push_integer(non_repeaters.into());
        buf.push_integer(max_repetitions.into());
        buf.push_integer(i64::from(req_id));
    });
}

pub(crate) fn build_get(
    version: Version,
    community: &[u8],
    req_id: i32,
    oid: &Oid,
    buf: &mut Buf,
    #[cfg(feature = "v3")] security: Option<&v3::Security>,
) -> Result<()> {
    build(
        version,
        community,
        snmp::MSG_GET,
        req_id,
        &[(oid, Value::Null)],
        0,
        0,
        buf,
        #[cfg(feature = "v3")]
        security,
    )
}

pub(crate) fn build_get_many(
    version: Version,
    community: &[u8],
    req_id: i32,
    oids: &[&Oid],
    buf: &mut Buf,
    #[cfg(feature = "v3")] security: Option<&v3::Security>,
) -> Result<()> {
    build(
        version,
        community,
        snmp::MSG_GET,
        req_id,
        oids.iter()
            .map(|&oid| (oid, Value::Null))
            .collect::<Vec<_>>()
            .as_slice(),
        0,
        0,
        buf,
        #[cfg(feature = "v3")]
        security,
    )
}

pub(crate) fn build_getnext(
    version: Version,
    community: &[u8],
    req_id: i32,
    oid: &Oid,
    buf: &mut Buf,
    #[cfg(feature = "v3")] security: Option<&v3::Security>,
) -> Result<()> {
    build(
        version,
        community,
        snmp::MSG_GET_NEXT,
        req_id,
        &[(oid, Value::Null)],
        0,
        0,
        buf,
        #[cfg(feature = "v3")]
        security,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_getbulk(
    version: Version,
    community: &[u8],
    req_id: i32,
    oids: &[&Oid],
    non_repeaters: u32,
    max_repetitions: u32,
    buf: &mut Buf,
    #[cfg(feature = "v3")] security: Option<&v3::Security>,
) -> Result<()> {
    build(
        version,
        community,
        snmp::MSG_GET_BULK,
        req_id,
        oids.iter()
            .map(|&oid| (oid, Value::Null))
            .collect::<Vec<_>>()
            .as_slice(),
        non_repeaters,
        max_repetitions,
        buf,
        #[cfg(feature = "v3")]
        security,
    )
}

pub(crate) fn build_set(
    version: Version,
    community: &[u8],
    req_id: i32,
    values: &[(&Oid, Value)],
    buf: &mut Buf,
    #[cfg(feature = "v3")] security: Option<&v3::Security>,
) -> Result<()> {
    build(
        version,
        community,
        snmp::MSG_SET,
        req_id,
        values,
        0,
        0,
        buf,
        #[cfg(feature = "v3")]
        security,
    )
}

#[derive(Debug, Clone)]
pub struct Pdu<'a> {
    pub(crate) version: i64,
    pub community: &'a [u8],
    pub message_type: MessageType,
    pub req_id: i32,
    pub error_status: u32,
    pub error_index: u32,
    pub varbinds: Varbinds<'a>,
    pub v1_trap_info: Option<V1TrapInfo<'a>>,
    #[cfg(feature = "v3")]
    pub v3_msg_id: i32,
}

#[derive(Debug, Clone)]
pub struct V1TrapInfo<'a> {
    pub enterprise: Oid<'a>,
    pub agent_addr: IpAddr,
    pub generic_trap: i64,
    pub specific_trap: i64,
    pub timestamp: u32,
}

/// SCANOPY LOCAL PATCH (GH #848) — see vendor/snmp2/README.md.
///
/// The id a datagram answers, read without parsing the rest of it: the request-id of a v1 or v2c
/// PDU, the msgID of a v3 message. `build` and `build_init` send a v3 request's msgID equal to its
/// request-id, and RFC 3412 §7.1 has the agent return it, so both name the request. The msgID is
/// in the plaintext header, which is what lets a v3 answer be recognised before it is
/// authenticated or decrypted, and so before it can move the engine clock.
///
/// `None` for anything that does not read as far as an id.
#[cfg(feature = "tokio")]
pub(crate) fn peek_response_id(bytes: &[u8]) -> Option<i32> {
    let seq = AsnReader::from_bytes(bytes)
        .read_raw(asn1::TYPE_SEQUENCE)
        .ok()?;
    let mut rdr = AsnReader::from_bytes(seq);
    let version = rdr.read_asn_integer().ok()?;
    let id = if version == Version::V3 as i64 {
        let global_data = rdr.read_raw(asn1::TYPE_SEQUENCE).ok()?;
        AsnReader::from_bytes(global_data).read_asn_integer().ok()?
    } else {
        rdr.read_asn_octetstring().ok()?;
        let ident = rdr.peek_byte().ok()?;
        AsnReader::from_bytes(rdr.read_raw(ident).ok()?)
            .read_asn_integer()
            .ok()?
    };
    i32::try_from(id).ok()
}

/// SCANOPY LOCAL PATCH (GH #848) — see vendor/snmp2/README.md.
///
/// Whether `id` is one a session issued before `current`, given the `first` id it ever issued.
/// Ids are issued in sequence and wrap, so the earlier ones are the `current - first` ids behind
/// `current`. Anything else, an id ahead of `current` or behind `first`, was never sent.
#[cfg(feature = "tokio")]
pub(crate) fn is_earlier_request(first: i32, current: i32, id: i32) -> bool {
    let behind = current.wrapping_sub(id) as u32;
    let issued = current.wrapping_sub(first) as u32;
    behind >= 1 && behind <= issued
}

impl<'a> Pdu<'a> {
    pub fn version(&self) -> Result<Version> {
        self.version.try_into()
    }

    fn parse_trap_v1(mut rdr: AsnReader<'a>, version: i64, community: &'a [u8]) -> Result<Pdu<'a>> {
        if version != Version::V1 as i64 {
            return Err(Error::AsnWrongType);
        }
        let oid = rdr.read_asn_objectidentifier()?;
        let addr = IpAddr::V4(Ipv4Addr::from(rdr.read_snmp_ipaddress()?));
        let generic_type = rdr.read_asn_integer()?;
        let specific_code = rdr.read_asn_integer()?;
        let timestamp = rdr.read_snmp_timeticks()?;
        let varbind_bytes = rdr.read_raw(asn1::TYPE_SEQUENCE)?;
        let varbinds = Varbinds::from_bytes(varbind_bytes);
        Ok(Pdu {
            version,
            community,
            message_type: MessageType::TrapV1,
            req_id: 0,
            error_status: 0,
            error_index: 0,
            varbinds,
            v1_trap_info: Some(V1TrapInfo {
                enterprise: oid,
                agent_addr: addr,
                generic_trap: generic_type,
                specific_trap: specific_code,
                timestamp,
            }),
            #[cfg(feature = "v3")]
            v3_msg_id: 0,
        })
    }

    pub fn from_bytes(bytes: &'a [u8]) -> Result<Pdu<'a>> {
        Self::from_bytes_inner(
            bytes,
            #[cfg(feature = "v3")]
            None,
        )
    }

    #[cfg(feature = "v3")]
    pub fn from_bytes_with_security(
        bytes: &'a [u8],
        security: Option<&'a mut v3::Security>,
    ) -> Result<Pdu<'a>> {
        Self::from_bytes_inner(bytes, security)
    }

    pub(crate) fn from_bytes_inner(
        bytes: &'a [u8],
        #[cfg(feature = "v3")] security: Option<&'a mut v3::Security>,
    ) -> Result<Pdu<'a>> {
        let seq = AsnReader::from_bytes(bytes).read_raw(asn1::TYPE_SEQUENCE)?;
        let mut rdr = AsnReader::from_bytes(seq);
        let version = rdr.read_asn_integer()?;
        if version != Version::V1 as i64
            && version != Version::V2C as i64
            && version != Version::V3 as i64
        {
            return Err(Error::UnsupportedVersion);
        }

        if version == Version::V3 as i64 {
            #[cfg(feature = "v3")]
            {
                if let Some(security) = security {
                    return Self::parse_v3(bytes, rdr, security);
                }
                return Err(Error::AuthFailure(v3::AuthErrorKind::SecurityNotProvided));
            }
            #[cfg(not(feature = "v3"))]
            {
                return Err(Error::UnsupportedVersion);
            }
        }

        let community = rdr.read_asn_octetstring()?;

        let ident = rdr.peek_byte()?;
        let message_type = MessageType::from_ident(ident)?;

        let mut response_pdu = AsnReader::from_bytes(rdr.read_raw(ident)?);

        if message_type == MessageType::TrapV1 {
            return Self::parse_trap_v1(response_pdu, version, community);
        }

        let req_id = response_pdu.read_asn_integer()?;
        if req_id < i64::from(i32::MIN) || req_id > i64::from(i32::MAX) {
            return Err(Error::ValueOutOfRange);
        }

        let error_status = response_pdu.read_asn_integer()?;
        if error_status < 0 || error_status > i64::from(i32::MAX) {
            return Err(Error::ValueOutOfRange);
        }

        let error_index = response_pdu.read_asn_integer()?;
        if error_index < 0 || error_index > i64::from(i32::MAX) {
            return Err(Error::ValueOutOfRange);
        }

        let varbind_bytes = response_pdu.read_raw(asn1::TYPE_SEQUENCE)?;
        let varbinds = Varbinds::from_bytes(varbind_bytes);

        Ok(Pdu {
            version,
            community,
            message_type,
            req_id: i32::try_from(req_id)?,
            error_status: u32::try_from(error_status)?,
            error_index: u32::try_from(error_index)?,
            varbinds,
            v1_trap_info: None,
            #[cfg(feature = "v3")]
            v3_msg_id: 0,
        })
    }

    pub(crate) fn validate(
        &self,
        expected_type: MessageType,
        expected_req_id: i32,
        expected_community: &[u8],
    ) -> Result<()> {
        if self.message_type != expected_type {
            return Err(Error::AsnWrongType);
        }
        if self.req_id != expected_req_id {
            return Err(Error::RequestIdMismatch);
        }
        if self.community != expected_community {
            return Err(Error::CommunityMismatch);
        }
        Ok(())
    }

    #[cfg(feature = "v3")]
    pub fn to_bytes_with_security(&self, security: Option<&v3::Security>) -> Result<Vec<u8>> {
        if self.version == Version::V3 as i64 {
            let mut buf = Buf::default();
            let ident = match self.message_type {
                MessageType::GetRequest => snmp::MSG_GET,
                MessageType::GetNextRequest => snmp::MSG_GET_NEXT,
                MessageType::GetBulkRequest => snmp::MSG_GET_BULK,
                MessageType::Response => snmp::MSG_RESPONSE,
                MessageType::SetRequest => snmp::MSG_SET,
                MessageType::InformRequest => snmp::MSG_INFORM,
                MessageType::Trap => snmp::MSG_TRAP,
                MessageType::TrapV1 => snmp::MSG_TRAP_V1,
                MessageType::Report => snmp::MSG_REPORT,
            };

            let varbind_pairs: Vec<(Oid, Value)> = self.varbinds.clone().collect();
            let (oids, values): (Vec<Oid>, Vec<Value>) = varbind_pairs.into_iter().unzip();
            let values_ref: Vec<(&Oid, Value)> = oids.iter().zip(values).collect();

            v3::build(
                ident,
                self.req_id,
                &values_ref,
                self.error_status,
                self.error_index,
                &mut buf,
                security,
            )?;
            Ok(buf.to_vec())
        } else {
            self.to_bytes()
        }
    }

    /// Convert PDU to bytes for UDP communication
    /// Returns a byte slice that can be sent over the network
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut buf = Buf::default();

        #[cfg(feature = "v3")]
        if self.version == Version::V3 as i64 {
            return Err(Error::AsnUnsupportedType); // V3 requires special handling
        }

        // Collect varbinds into a Vec for processing
        let varbind_pairs: Vec<(Oid, Value)> = self.varbinds.clone().collect();
        let (oids, values): (Vec<Oid>, Vec<Value>) = varbind_pairs.into_iter().unzip();
        let values_ref: Vec<(&Oid, Value)> = oids.iter().zip(values).collect();

        // Special handling for TrapV1
        if self.message_type == MessageType::TrapV1 {
            if let Some(ref trap_info) = self.v1_trap_info {
                buf.reset();
                buf.push_sequence(|buf| {
                    buf.push_constructed(snmp::MSG_TRAP_V1, |buf| {
                        push_varbinds(buf, &values_ref);
                        // Timestamp
                        buf.push_timeticks(trap_info.timestamp);
                        // Specific trap
                        buf.push_integer(trap_info.specific_trap);
                        // Generic trap
                        buf.push_integer(trap_info.generic_trap);
                        // Agent address
                        if let IpAddr::V4(ipv4) = trap_info.agent_addr {
                            buf.push_ipaddress(ipv4.octets());
                        }
                        // Enterprise OID
                        buf.push_object_identifier_raw(trap_info.enterprise.as_bytes());
                    });
                    buf.push_octet_string(self.community);
                    buf.push_integer(self.version);
                });
                return Ok(buf.to_vec());
            }
            return Err(Error::AsnWrongType);
        }

        // Determine the message other identifier
        let ident = match self.message_type {
            MessageType::GetRequest => snmp::MSG_GET,
            MessageType::GetNextRequest => snmp::MSG_GET_NEXT,
            MessageType::GetBulkRequest => snmp::MSG_GET_BULK,
            MessageType::Response => snmp::MSG_RESPONSE,
            MessageType::SetRequest => snmp::MSG_SET,
            MessageType::InformRequest => snmp::MSG_INFORM,
            MessageType::Trap => snmp::MSG_TRAP,
            MessageType::Report => snmp::MSG_REPORT,
            MessageType::TrapV1 => unreachable!(),
        };

        build(
            self.version()?,
            self.community,
            ident,
            self.req_id,
            &values_ref,
            self.error_status,
            self.error_index,
            &mut buf,
            #[cfg(feature = "v3")]
            None,
        )?;

        Ok(buf.to_vec())
    }

    /// Get a reference to the underlying bytes (for zero-copy scenarios)
    /// Note: This creates a new Vec, for true zero-copy, use `to_bytes` carefully
    pub fn as_bytes(&self) -> Result<Vec<u8>> {
        self.to_bytes()
    }
}
