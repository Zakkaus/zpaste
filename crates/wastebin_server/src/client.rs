//! The client address of a request and the per-client rate of password key derivations
//! (architecture invariants 10 and 11).

use std::collections::HashMap;
use std::convert::Infallible;
use std::net::{IpAddr, Ipv6Addr};
use std::num::NonZeroU32;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::connect_info::Connected;
use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::request::Parts;
use axum::serve::IncomingStream;
use tokio::net::{TcpListener, UnixListener};

use crate::AppState;

/// Header Cloudflare sets to the address of the client it serves.
const CF_CONNECTING_IP: &str = "cf-connecting-ip";

/// Buckets kept before full ones are dropped; a full bucket holds no state worth keeping.
const PRUNE_AT: usize = 4096;

/// The peer of a connection: its address over TCP, `None` over a Unix socket.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Peer(pub Option<IpAddr>);

impl Connected<IncomingStream<'_, TcpListener>> for Peer {
    fn connect_info(stream: IncomingStream<'_, TcpListener>) -> Self {
        Self(Some(stream.remote_addr().ip()))
    }
}

impl Connected<IncomingStream<'_, UnixListener>> for Peer {
    fn connect_info(_stream: IncomingStream<'_, UnixListener>) -> Self {
        Self(None)
    }
}

/// An address range in CIDR notation; a bare address is a range of one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Cidr {
    network: IpAddr,
    prefix: u8,
}

impl Cidr {
    fn contains(self, ip: IpAddr) -> bool {
        match (self.network, ip.to_canonical()) {
            (IpAddr::V4(network), IpAddr::V4(ip)) => {
                let mask = u32::MAX
                    .checked_shl(32 - u32::from(self.prefix))
                    .unwrap_or(0);
                u32::from(network) & mask == u32::from(ip) & mask
            }
            (IpAddr::V6(network), IpAddr::V6(ip)) => {
                let mask = u128::MAX
                    .checked_shl(128 - u32::from(self.prefix))
                    .unwrap_or(0);
                u128::from(network) & mask == u128::from(ip) & mask
            }
            _ => false,
        }
    }
}

impl FromStr for Cidr {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (address, prefix) = value.split_once('/').unwrap_or((value, ""));
        let network = address
            .parse::<IpAddr>()
            .map_err(|_| format!("`{value}` is not an address or CIDR range"))?
            .to_canonical();
        let bits = if network.is_ipv4() { 32 } else { 128 };
        let prefix = if prefix.is_empty() {
            bits
        } else {
            prefix
                .parse::<u8>()
                .ok()
                .filter(|prefix| *prefix <= bits)
                .ok_or_else(|| format!("`{value}` has an invalid prefix length"))?
        };

        Ok(Self { network, prefix })
    }
}

/// The proxies whose `CF-Connecting-IP` header is believed (`WASTEBIN_TRUSTED_PROXIES`). Empty by
/// default: every request is keyed by its own connection.
#[derive(Clone, Debug, Default)]
pub(crate) struct TrustedProxies {
    ranges: Vec<Cidr>,
    /// Whether a connection over the Unix socket is a trusted proxy (the entry `unix`).
    unix: bool,
}

impl TrustedProxies {
    fn trusts(&self, peer: Peer) -> bool {
        match peer.0 {
            Some(ip) => self.ranges.iter().any(|range| range.contains(ip)),
            None => self.unix,
        }
    }
}

impl FromStr for TrustedProxies {
    type Err = String;

    /// Comma-separated CIDR ranges or addresses, and `unix` for the Unix socket.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut proxies = Self::default();
        for entry in value
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
        {
            if entry == "unix" {
                proxies.unix = true;
            } else {
                proxies.ranges.push(entry.parse()?);
            }
        }
        Ok(proxies)
    }
}

/// Who a request comes from, for rate limiting: an IPv4 address, the /64 network of an IPv6
/// address (one subscriber usually holds a whole /64), or `None` for an untrusted Unix-socket
/// peer, which has no address and so shares one key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ClientKey(Option<IpAddr>);

impl ClientKey {
    fn new(ip: Option<IpAddr>) -> Self {
        Self(ip.map(|ip| match ip.to_canonical() {
            IpAddr::V6(ip) => IpAddr::V6(Ipv6Addr::from(u128::from(ip) & !u128::from(u64::MAX))),
            ip @ IpAddr::V4(_) => ip,
        }))
    }

    /// `CF-Connecting-IP` when the connection comes from a trusted proxy and the header holds one
    /// address; the connection's own address otherwise. No other forwarding header is read.
    fn of(peer: Peer, headers: &axum::http::HeaderMap, proxies: &TrustedProxies) -> Self {
        let forwarded = proxies
            .trusts(peer)
            .then(|| headers.get(CF_CONNECTING_IP))
            .flatten()
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.trim().parse::<IpAddr>().ok());

        Self::new(forwarded.or(peer.0))
    }
}

impl FromRequestParts<AppState> for ClientKey {
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Infallible> {
        // A router served without connect info (none in this crate) counts as one unknown peer.
        let peer = parts
            .extensions
            .get::<ConnectInfo<Peer>>()
            .map_or(Peer(None), |ConnectInfo(peer)| *peer);

        Ok(Self::of(peer, &parts.headers, &state.proxies))
    }
}

/// Token bucket per [`ClientKey`] for password key derivations (`WASTEBIN_KDF_RATE` per minute,
/// the same number as the burst), kept as a theoretical arrival time (GCRA) so it needs no
/// floating point. It sits in front of the process-wide [`wastebin_core::crypto::KdfLimit`]: this
/// bounds what one client may ask for, that bounds the memory all of them use at once.
#[derive(Clone)]
pub(crate) struct RateLimit {
    /// When each client's bucket is full again.
    full_at: Arc<Mutex<HashMap<ClientKey, Instant>>>,
    /// Time to earn one token back.
    interval: Duration,
    /// How far `full_at` may run ahead of now: the burst less one token.
    tolerance: Duration,
}

impl RateLimit {
    pub(crate) const DEFAULT_PER_MINUTE: NonZeroU32 = NonZeroU32::new(10).expect("10 is non-zero");

    pub(crate) fn per_minute(rate: NonZeroU32) -> Self {
        let interval = Duration::from_secs(60) / rate.get();
        Self {
            full_at: Arc::default(),
            interval,
            tolerance: interval * (rate.get() - 1),
        }
    }

    /// Take one token for `client`, or answer how long until one is free.
    pub(crate) fn take(&self, client: ClientKey) -> Result<(), Duration> {
        self.take_at(client, Instant::now())
    }

    fn take_at(&self, client: ClientKey, now: Instant) -> Result<(), Duration> {
        let mut full_at = self
            .full_at
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        if full_at.len() >= PRUNE_AT {
            full_at.retain(|_, at| *at > now);
        }

        let at = full_at.get(&client).map_or(now, |at| (*at).max(now));
        let ahead = at - now;
        if ahead > self.tolerance {
            return Err(ahead - self.tolerance);
        }

        full_at.insert(client, at + self.interval);
        Ok(())
    }
}

impl Default for RateLimit {
    fn default() -> Self {
        Self::per_minute(Self::DEFAULT_PER_MINUTE)
    }
}

/// `Retry-After` in whole seconds, rounded up and at least 1.
pub(crate) fn retry_after_secs(wait: Duration) -> u64 {
    (wait.as_secs() + u64::from(wait.subsec_nanos() > 0)).max(1)
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderMap;

    use super::*;

    fn headers(cf: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(CF_CONNECTING_IP, cf.parse().expect("header value"));
        headers
    }

    fn key(ip: &str) -> ClientKey {
        ClientKey::new(Some(ip.parse().expect("address")))
    }

    #[test]
    fn header_counts_only_from_a_trusted_proxy() -> Result<(), String> {
        let proxies: TrustedProxies = "173.245.48.0/20, 172.18.0.0/16".parse()?;
        let spoofed = headers("198.51.100.7");

        let direct = Peer(Some("203.0.113.9".parse().expect("address")));
        assert_eq!(
            ClientKey::of(direct, &spoofed, &proxies),
            key("203.0.113.9")
        );

        let traefik = Peer(Some("172.18.0.3".parse().expect("address")));
        assert_eq!(
            ClientKey::of(traefik, &spoofed, &proxies),
            key("198.51.100.7")
        );
        // A trusted peer without a usable header is keyed by itself.
        assert_eq!(
            ClientKey::of(traefik, &headers("not an address"), &proxies),
            key("172.18.0.3")
        );
        assert_eq!(
            ClientKey::of(traefik, &HeaderMap::new(), &proxies),
            key("172.18.0.3")
        );

        // No proxies by default; the Unix socket is trusted only when listed.
        let none = TrustedProxies::default();
        assert_eq!(ClientKey::of(traefik, &spoofed, &none), key("172.18.0.3"));
        assert_eq!(ClientKey::of(Peer(None), &spoofed, &none), ClientKey(None));
        let unix: TrustedProxies = "unix".parse()?;
        assert_eq!(
            ClientKey::of(Peer(None), &spoofed, &unix),
            key("198.51.100.7")
        );

        Ok(())
    }

    #[test]
    fn ranges_parse_and_match() -> Result<(), String> {
        let proxies: TrustedProxies = "10.0.0.0/8,2400:cb00::/32,192.0.2.1".parse()?;
        for (ip, trusted) in [
            ("10.255.0.1", true),
            ("11.0.0.1", false),
            ("::ffff:10.1.2.3", true),
            ("2400:cb00:1::1", true),
            ("2400:cb01::1", false),
            ("192.0.2.1", true),
            ("192.0.2.2", false),
        ] {
            let peer = Peer(Some(ip.parse().expect("address")));
            assert_eq!(proxies.trusts(peer), trusted, "{ip}");
        }

        for invalid in ["10.0.0.0/33", "example.org", "::/129", "10.0.0.0/x"] {
            assert!(invalid.parse::<TrustedProxies>().is_err(), "{invalid}");
        }

        Ok(())
    }

    #[test]
    fn ipv6_clients_share_their_64() {
        assert_eq!(key("2001:db8:1:2::1"), key("2001:db8:1:2:ffff::9"));
        assert_ne!(key("2001:db8:1:2::1"), key("2001:db8:1:3::1"));
    }

    #[test]
    fn bucket_allows_the_burst_then_refills() {
        let limit = RateLimit::per_minute(NonZeroU32::new(2).expect("non-zero"));
        let client = key("192.0.2.1");
        let start = Instant::now();

        assert!(limit.take_at(client, start).is_ok());
        assert!(limit.take_at(client, start).is_ok());
        let wait = limit.take_at(client, start).expect_err("burst spent");
        assert_eq!(wait, Duration::from_secs(30));
        // Another client has its own bucket.
        assert!(limit.take_at(key("192.0.2.2"), start).is_ok());

        let later = start + Duration::from_secs(20);
        assert_eq!(
            limit.take_at(client, later).expect_err("not refilled"),
            Duration::from_secs(10)
        );
        assert!(
            limit
                .take_at(client, start + Duration::from_secs(30))
                .is_ok()
        );
        assert!(
            limit
                .take_at(client, start + Duration::from_secs(30))
                .is_err()
        );
    }

    #[test]
    fn full_buckets_are_dropped_once_many_are_kept() {
        let limit = RateLimit::per_minute(NonZeroU32::new(60).expect("non-zero"));
        let start = Instant::now();
        for n in 0..PRUNE_AT {
            let ip = IpAddr::from(u32::try_from(n).expect("small").to_be_bytes());
            assert!(limit.take_at(ClientKey::new(Some(ip)), start).is_ok());
        }
        let busy = key("192.0.2.200");
        for _ in 0..60 {
            assert!(limit.take_at(busy, start).is_ok());
        }

        // A second later every bucket but the busy one is full again and is dropped.
        let later = start + Duration::from_secs(1);
        assert!(limit.take_at(key("192.0.2.201"), later).is_ok());
        assert_eq!(limit.full_at.lock().expect("lock").len(), 2);
        assert!(limit.take_at(busy, later).is_ok());
        assert!(limit.take_at(busy, later).is_err());
    }

    #[test]
    fn retry_after_rounds_up() {
        assert_eq!(retry_after_secs(Duration::ZERO), 1);
        assert_eq!(retry_after_secs(Duration::from_millis(1)), 1);
        assert_eq!(retry_after_secs(Duration::from_secs(6)), 6);
        assert_eq!(retry_after_secs(Duration::from_millis(6001)), 7);
    }
}
