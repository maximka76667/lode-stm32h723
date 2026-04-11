use core::fmt::Write as _;

use embassy_net::{Runner, Stack, StackResources, tcp::TcpSocket};
use embassy_stm32::{
    eth::{Ethernet, GenericPhy, PacketQueue, Sma},
    peripherals,
};
use embassy_time::Duration;
use embedded_io_async::Write;
use heapless::String;
use static_cell::StaticCell;

use crate::bme280::Measurements;

// Change these to match your machine
pub const SERVER_ADDR: [u8; 4] = [192, 168, 1, 136];
pub const SERVER_HOST: &str = "192.168.1.136";
pub const SERVER_PORT: u16 = 3111;

pub type Device = Ethernet<'static, peripherals::ETH, GenericPhy<Sma<'static, peripherals::ETH_SMA>>>;

static PACKETS: StaticCell<PacketQueue<4, 4>> = StaticCell::new();
static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();

pub fn packet_queue() -> &'static mut PacketQueue<4, 4> {
    PACKETS.init(PacketQueue::new())
}

pub fn init_stack(device: Device, seed: u64) -> (Stack<'static>, Runner<'static, Device>) {
    embassy_net::new(
        device,
        embassy_net::Config::dhcpv4(Default::default()),
        RESOURCES.init(StackResources::new()),
        seed,
    )
}

#[embassy_executor::task]
pub async fn net_task(mut runner: Runner<'static, Device>) -> ! {
    runner.run().await
}

/// Returns true if the reading was sent successfully.
pub async fn send_reading(stack: Stack<'static>, m: &Measurements) -> bool {
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

    let mut request: String<384> = String::new();
    write!(
        request,
        "POST /readings HTTP/1.1\r\nHost: {}:{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        SERVER_HOST, SERVER_PORT, body.len(), body
    )
    .unwrap();

    let mut rx_buf = [0u8; 256];
    let mut tx_buf = [0u8; 256];
    let mut socket = TcpSocket::new(stack, &mut rx_buf, &mut tx_buf);
    socket.set_timeout(Some(Duration::from_secs(5)));

    let addr = embassy_net::IpAddress::v4(
        SERVER_ADDR[0], SERVER_ADDR[1], SERVER_ADDR[2], SERVER_ADDR[3],
    );

    let ok = if socket.connect((addr, SERVER_PORT)).await.is_ok() {
        let sent = socket.write_all(request.as_bytes()).await.is_ok();
        socket.flush().await.ok();
        let mut resp = [0u8; 64];
        socket.read(&mut resp).await.ok();
        sent
    } else {
        false
    };

    socket.abort();
    ok
}
