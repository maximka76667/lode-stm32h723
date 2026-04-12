use core::cell::UnsafeCell;
use core::fmt::Write as _;

use embassy_net::Stack;
use embassy_net::dns::DnsSocket;
use embassy_net::tcp::client::{TcpClient, TcpClientState};
use heapless::String;
use reqwless::client::{HttpClient, TlsConfig, TlsVerify};
use reqwless::headers::ContentType;
use reqwless::request::{Method, RequestBuilder};

use crate::bme280::Measurements;

pub const HOST: &str = "lode-api-rust.onrender.com";
pub const URL: &str = "https://lode-api-rust.onrender.com/readings";

// ---------------------------------------------------------------------------
// Safe wrappers around statics that require unsafe to construct
// ---------------------------------------------------------------------------

/// Newtype that makes `TcpClientState` shareable as a static.
///
/// # Safety
/// Sound only on single-core targets (STM32H723 is single-core) where
/// `send_reading` is never called concurrently.
struct SyncTcpState(TcpClientState<1, 4096, 4096>);
// SAFETY: see above.
unsafe impl Sync for SyncTcpState {}

static TCP_STATE: SyncTcpState = SyncTcpState(TcpClientState::new());

/// Owns the two TLS record buffers and hands out `&mut` slices.
///
/// # Safety invariant
/// `get()` must only be called from a single context at a time — the caller
/// guarantees exclusive access. On this MCU that is always true because
/// `send_reading` is awaited sequentially and never re-entered.
struct TlsBuffers {
    read: UnsafeCell<[u8; 16384]>,
    write: UnsafeCell<[u8; 4096]>,
}

// SAFETY: single-core target; `get()` is never called concurrently.
unsafe impl Sync for TlsBuffers {}

impl TlsBuffers {
    const fn new() -> Self {
        Self {
            read: UnsafeCell::new([0u8; 16384]),
            write: UnsafeCell::new([0u8; 4096]),
        }
    }

    /// Returns exclusive mutable references to both buffers.
    /// Caller must ensure no other reference to these buffers is live.
    fn get(&self) -> (&mut [u8], &mut [u8]) {
        // SAFETY: upheld by the safety invariant on the type.
        unsafe { (&mut *self.read.get(), &mut *self.write.get()) }
    }
}

static TLS_BUFFERS: TlsBuffers = TlsBuffers::new();

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// POST a BME280 reading to the backend over HTTPS.
/// `seed` must be unique per call — pass an incrementing counter seeded from
/// the hardware RNG so each TLS session uses a distinct PRNG state.
pub async fn send_reading(stack: Stack<'static>, seed: u64, m: &Measurements) -> bool {
    let temp_int = m.temperature / 100;
    let temp_frac = m.temperature.unsigned_abs() % 100;
    let press_pa = m.pressure / 256;
    let press_int = press_pa / 100;
    let press_frac = press_pa % 100;
    let hum_int = m.humidity / 1024;
    let hum_frac = (m.humidity % 1024) * 100 / 1024;

    let mut body: String<128> = String::new();
    write!(
        body,
        r#"{{"temperature_c":{}.{:02},"humidity_pct":{}.{:02},"pressure_hpa":{}.{:02}}}"#,
        temp_int, temp_frac, hum_int, hum_frac, press_int, press_frac
    )
    .unwrap();

    let tcp = TcpClient::new(stack, &TCP_STATE.0);
    let dns = DnsSocket::new(stack);
    let (read_buf, write_buf) = TLS_BUFFERS.get();

    let tls = TlsConfig::new(seed, read_buf, write_buf, TlsVerify::None);
    let mut client = HttpClient::new_with_tls(&tcp, &dns, tls);

    let mut rx_buf = [0u8; 1024];
    let Ok(req) = client.request(Method::POST, URL).await else {
        return false;
    };

    req.body(body.as_bytes())
        .content_type(ContentType::ApplicationJson)
        .send(&mut rx_buf)
        .await
        .is_ok()
}
