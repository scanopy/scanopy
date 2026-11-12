use std::{
    io,
    net::{Ipv4Addr, Ipv6Addr, SocketAddr},
    num::Wrapping,
};

use crate::{
    BUFFER_SIZE, Error, MessageType, Oid, Result, Value, Version,
    pdu::{self, Pdu},
};
use tokio::net::{ToSocketAddrs, UdpSocket, lookup_host};

#[cfg(feature = "v3")]
use crate::v3;

/// Asynchronous SNMP client
pub struct AsyncSession {
    version: Version,
    socket: UdpSocket,
    community: Vec<u8>,
    /// SCANOPY LOCAL PATCH — see vendor/snmp2/README.md.
    ///
    /// Incremented the moment an id is taken for a request, never after the response arrives.
    /// A request whose future is dropped mid-`recv` (the caller timed out) or that fails to decode
    /// must still burn its id: otherwise the next request reuses it, and the previous request's
    /// queued response validates against it — leaving the session permanently one answer behind.
    req_id: Wrapping<i32>,
    /// SCANOPY LOCAL PATCH (GH #848): the first id this session issued. With `req_id` it bounds
    /// the ids of the requests already sent, which is how a late answer to one of them is told
    /// from an answer to nothing. See `send_and_recv`.
    first_req_id: i32,
    send_pdu: pdu::Buf,
    #[cfg(not(feature = "heap_buffers"))]
    recv_buf: [u8; BUFFER_SIZE],
    #[cfg(feature = "heap_buffers")]
    recv_buf: Box<[u8]>,
    #[cfg(feature = "v3")]
    security: Option<v3::Security>,
}

impl AsyncSession {
    pub async fn new_v1<SA>(
        destination: SA,
        community: &[u8],
        starting_req_id: i32,
    ) -> io::Result<Self>
    where
        SA: ToSocketAddrs,
    {
        Self::new(Version::V1, destination, community, starting_req_id).await
    }

    pub async fn new_v2c<SA>(
        destination: SA,
        community: &[u8],
        starting_req_id: i32,
    ) -> io::Result<Self>
    where
        SA: ToSocketAddrs,
    {
        Self::new(Version::V2C, destination, community, starting_req_id).await
    }

    #[cfg(feature = "v3")]
    pub async fn new_v3<SA>(
        destination: SA,
        starting_req_id: i32,
        security: v3::Security,
    ) -> io::Result<Self>
    where
        SA: ToSocketAddrs,
    {
        let mut session = Self::new(Version::V3, destination, &[], starting_req_id).await?;
        session.community.clone_from(&security.username);
        session.security = Some(security);
        Ok(session)
    }

    async fn new<SA>(
        version: Version,
        destination: SA,
        community: &[u8],
        starting_req_id: i32,
    ) -> io::Result<Self>
    where
        SA: ToSocketAddrs,
    {
        let socket = match lookup_host(destination).await?.next() {
            Some(SocketAddr::V4(addr)) => {
                let s = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).await?;
                s.connect(addr).await?;
                s
            }
            Some(SocketAddr::V6(addr)) => {
                let s = UdpSocket::bind((Ipv6Addr::UNSPECIFIED, 0)).await?;
                s.connect(addr).await?;
                s
            }
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "No address found",
                ));
            }
        };
        Ok(Self {
            version,
            socket,
            community: community.to_vec(),
            req_id: Wrapping(starting_req_id),
            first_req_id: starting_req_id,
            send_pdu: pdu::Buf::default(),
            #[cfg(not(feature = "heap_buffers"))]
            #[allow(clippy::large_stack_arrays)]
            recv_buf: [0; BUFFER_SIZE],
            #[cfg(feature = "heap_buffers")]
            recv_buf: vec![0u8; BUFFER_SIZE].into_boxed_slice(),
            #[cfg(feature = "v3")]
            security: None,
        })
    }

    /// Returns the version of the session, e.g. Version::V1.
    /// This is useful for determining whether bulk get is supported.
    pub fn version(&self) -> Version {
        self.version
    }

    #[cfg(not(feature = "v3"))]
    #[allow(clippy::unused_self, clippy::unused_async)]
    pub async fn init(&mut self) -> Result<()> {
        Ok(())
    }

    #[cfg(feature = "v3")]
    pub async fn init(&mut self) -> Result<()> {
        self.discover_engine().await?;
        self.synchronize_time().await
    }

    /// SCANOPY LOCAL PATCH (GH #846): RFC 3414 §4's first step, split out of `init` so a caller
    /// can time the two steps apart. This one is unauthenticated: any SNMPv3 engine answers it,
    /// whatever the user or keys. See vendor/snmp2/README.md.
    #[cfg(feature = "v3")]
    pub async fn discover_engine(&mut self) -> Result<()> {
        if let Some(ref mut security) = self.security {
            security.reset_engine_id();
            security.reset_engine_counters();
            // send a request to get the engine id
            let req_id = self.req_id.0;
            self.req_id += Wrapping(1);
            v3::build_init(req_id, &mut self.send_pdu);
            if let Err(e) = Pdu::from_bytes_inner(
                Self::send_and_recv(
                    &self.socket,
                    &self.send_pdu,
                    &mut self.recv_buf,
                    self.first_req_id,
                    req_id,
                )
                .await?,
                Some(security),
            ) && e != Error::AuthUpdated
            {
                return Err(e);
            }
            if security.need_init() {
                return Err(Error::AuthFailure(v3::AuthErrorKind::NotAuthenticated));
            }
        }
        Ok(())
    }

    /// The second step of `init`: the first request that uses the user's keys, encrypted when
    /// the user has a privacy key. A no-op for a user without authentication, or once the clock
    /// is synchronized.
    #[cfg(feature = "v3")]
    pub async fn synchronize_time(&mut self) -> Result<()> {
        if let Some(ref mut security) = self.security {
            // SCANOPY LOCAL PATCH (GH #813): RFC 3414 §4's second step. The discovery Report is
            // unauthenticated, so its boots and time are only a hint; an agent that sends none
            // answers the first authenticated request with a notInTimeWindow Report, and the
            // caller would see `AuthUpdated` instead of its answer. An authenticated GET with
            // no varbinds synchronizes the clock here, whichever way the agent answers.
            if security.need_auth() && !security.is_time_synchronized() {
                let req_id = self.req_id.0;
                self.req_id += Wrapping(1);
                v3::build(
                    crate::snmp::MSG_GET,
                    req_id,
                    &[],
                    0,
                    0,
                    &mut self.send_pdu,
                    Some(security),
                )?;
                if let Err(e) = Pdu::from_bytes_inner(
                    Self::send_and_recv(
                        &self.socket,
                        &self.send_pdu,
                        &mut self.recv_buf,
                        self.first_req_id,
                        req_id,
                    )
                    .await?,
                    Some(security),
                ) && e != Error::AuthUpdated
                {
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    /// Checks if KeyExtension affects this session privacy and then re-inits session with different KeyExtension
    ///
    /// # Returns
    /// 'Ok(Some(new_key_extension))' When new_key_extension method was set
    /// 'Ok(None)' When security disabled
    /// or Auth type is not AuthPriv
    /// or when Auth-Priv pair is not the one that needs key extension
    /// or when KeyExtension was not set for the session.
    /// 'Err(error)' when 'init()' failed with error returned from 'init()'
    #[cfg(feature = "v3")]
    pub async fn try_another_key_extension_method(&mut self) -> Result<Option<v3::KeyExtension>> {
        if let Some(ref mut security) = self.security
            && let Some(new_method) = security.another_key_extension_method()
        {
            security.authoritative_state = v3::AuthoritativeState::default();
            self.init().await?;
            return Ok(Some(new_method));
        }
        Ok(None)
    }

    #[cfg(not(feature = "v3"))]
    #[allow(clippy::unused_self)]
    fn prepare(&mut self) {}

    #[cfg(feature = "v3")]
    fn prepare(&mut self) {
        if let Some(ref mut security) = self.security {
            security.correct_authoritative_engine_time();
        }
    }

    /// SCANOPY LOCAL PATCH — see vendor/snmp2/README.md.
    ///
    /// Discard any datagram left in the socket buffer by an earlier request.
    ///
    /// A request whose future is dropped mid-`recv` (the caller timed out) leaves its response
    /// queued. `send_and_recv` reads exactly one datagram, so without draining, the next request
    /// receives the *previous* request's answer and the session stays one response behind for the
    /// rest of its life. The socket is connected, so only the peer's datagrams are ever queued
    /// here, and a drained datagram is by definition one nobody is waiting for any more.
    fn drain_stale(socket: &UdpSocket, scratch: &mut [u8]) -> usize {
        let mut drained = 0;
        while socket.try_recv(scratch).is_ok() {
            drained += 1;
        }
        drained
    }

    /// SCANOPY LOCAL PATCH (GH #848) — see vendor/snmp2/README.md.
    ///
    /// Send the request with id `req_id` and return the datagram that answers it.
    ///
    /// `drain_stale` only removes what has already arrived. An agent that takes longer than the
    /// caller's timeout over one request answers it after the next request is sent, and that
    /// answer is the first datagram this request reads. Returned to the caller it fails
    /// validation, the caller sends another request, and that one reads the answer to this one:
    /// the session is one answer behind until it is dropped. So an answer to a request this
    /// session issued earlier is passed over and the read continues.
    ///
    /// There is no deadline here, as there was none before. The caller bounds the whole call,
    /// and a request whose answer never comes ends when that bound drops this future.
    ///
    /// A datagram whose id is not an earlier request's goes to the caller's `validate` as it
    /// always did, so an id this session never sent is still `RequestIdMismatch`.
    async fn send_and_recv<'a>(
        socket: &UdpSocket,
        pdu: &pdu::Buf,
        out: &'a mut [u8],
        first_req_id: i32,
        req_id: i32,
    ) -> Result<&'a [u8]> {
        Self::drain_stale(socket, out);
        if socket.send(pdu).await.is_err() {
            return Err(Error::Send);
        }
        let len = loop {
            let Ok(len) = socket.recv(out).await else {
                return Err(Error::Receive);
            };
            let late = pdu::peek_response_id(&out[..len])
                .is_some_and(|id| pdu::is_earlier_request(first_req_id, req_id, id));
            if !late {
                break len;
            }
        };
        Ok(&out[..len])
    }

    pub async fn get(&mut self, oid: &Oid<'_>) -> Result<Pdu<'_>> {
        self.prepare();
        let req_id = self.req_id.0;
        self.req_id += Wrapping(1);
        pdu::build_get(
            self.version,
            self.community.as_slice(),
            req_id,
            oid,
            &mut self.send_pdu,
            #[cfg(feature = "v3")]
            self.security.as_ref(),
        )?;
        let resp = Pdu::from_bytes_inner(
            Self::send_and_recv(
                &self.socket,
                &self.send_pdu,
                &mut self.recv_buf,
                self.first_req_id,
                req_id,
            )
            .await?,
            #[cfg(feature = "v3")]
            self.security.as_mut(),
        )?;
        resp.validate(MessageType::Response, req_id, &self.community)?;
        Ok(resp)
    }

    pub async fn get_many(&mut self, oids: &[&Oid<'_>]) -> Result<Pdu<'_>> {
        self.prepare();
        let req_id = self.req_id.0;
        self.req_id += Wrapping(1);
        pdu::build_get_many(
            self.version,
            self.community.as_slice(),
            req_id,
            oids,
            &mut self.send_pdu,
            #[cfg(feature = "v3")]
            self.security.as_ref(),
        )?;
        let resp = Pdu::from_bytes_inner(
            Self::send_and_recv(
                &self.socket,
                &self.send_pdu,
                &mut self.recv_buf,
                self.first_req_id,
                req_id,
            )
            .await?,
            #[cfg(feature = "v3")]
            self.security.as_mut(),
        )?;
        resp.validate(MessageType::Response, req_id, &self.community)?;
        Ok(resp)
    }

    pub async fn getnext(&mut self, oid: &Oid<'_>) -> Result<Pdu<'_>> {
        self.prepare();
        let req_id = self.req_id.0;
        self.req_id += Wrapping(1);
        pdu::build_getnext(
            self.version,
            self.community.as_slice(),
            req_id,
            oid,
            &mut self.send_pdu,
            #[cfg(feature = "v3")]
            self.security.as_ref(),
        )?;
        let resp = Pdu::from_bytes_inner(
            Self::send_and_recv(
                &self.socket,
                &self.send_pdu,
                &mut self.recv_buf,
                self.first_req_id,
                req_id,
            )
            .await?,
            #[cfg(feature = "v3")]
            self.security.as_mut(),
        )?;
        resp.validate(MessageType::Response, req_id, &self.community)?;
        Ok(resp)
    }

    pub async fn getbulk(
        &mut self,
        oids: &[&Oid<'_>],
        non_repeaters: u32,
        max_repetitions: u32,
    ) -> Result<Pdu<'_>> {
        self.prepare();
        let req_id = self.req_id.0;
        self.req_id += Wrapping(1);
        pdu::build_getbulk(
            self.version,
            self.community.as_slice(),
            req_id,
            oids,
            non_repeaters,
            max_repetitions,
            &mut self.send_pdu,
            #[cfg(feature = "v3")]
            self.security.as_ref(),
        )?;
        let resp = Pdu::from_bytes_inner(
            Self::send_and_recv(
                &self.socket,
                &self.send_pdu,
                &mut self.recv_buf,
                self.first_req_id,
                req_id,
            )
            .await?,
            #[cfg(feature = "v3")]
            self.security.as_mut(),
        )?;
        resp.validate(MessageType::Response, req_id, &self.community)?;
        Ok(resp)
    }

    pub async fn set(&mut self, values: &[(&Oid<'_>, Value<'_>)]) -> Result<Pdu<'_>> {
        self.prepare();
        let req_id = self.req_id.0;
        self.req_id += Wrapping(1);
        pdu::build_set(
            self.version,
            self.community.as_slice(),
            req_id,
            values,
            &mut self.send_pdu,
            #[cfg(feature = "v3")]
            self.security.as_ref(),
        )?;
        let resp = Pdu::from_bytes_inner(
            Self::send_and_recv(
                &self.socket,
                &self.send_pdu,
                &mut self.recv_buf,
                self.first_req_id,
                req_id,
            )
            .await?,
            #[cfg(feature = "v3")]
            self.security.as_mut(),
        )?;
        resp.validate(MessageType::Response, req_id, &self.community)?;
        Ok(resp)
    }
}
