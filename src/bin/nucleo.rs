#![no_std]
#![no_main]

#[path = "../fmt.rs"]
mod fmt;
use fmt::info;

use embassy_executor::Spawner;
use embassy_stm32::{
    bind_interrupts, dma,
    eth::{self, Ethernet},
    gpio::{Level, Output, Speed},
    i2c::{self, I2c},
    peripherals,
    rng::{self, Rng},
    time::Hertz,
    wdg::IndependentWatchdog,
};
use lode_stm32h723::{
    bme280::Bme280,
    leds::{self, BoardState},
    net,
};

#[cfg(not(feature = "defmt"))]
use panic_halt as _;
#[cfg(feature = "defmt")]
use {defmt_rtt as _, panic_probe as _};

const MAC_ADDR: [u8; 6] = [0x02, 0x00, 0x00, 0x00, 0x00, 0x01];

bind_interrupts!(struct Irqs {
    ETH => eth::InterruptHandler;
    RNG => rng::InterruptHandler<peripherals::RNG>;
    I2C2_EV => i2c::EventInterruptHandler<peripherals::I2C2>;
    I2C2_ER => i2c::ErrorInterruptHandler<peripherals::I2C2>;
    DMA1_STREAM4 => dma::InterruptHandler<peripherals::DMA1_CH4>;
    DMA1_STREAM5 => dma::InterruptHandler<peripherals::DMA1_CH5>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut config = embassy_stm32::Config::default();
    {
        use embassy_stm32::rcc::*;
        config.rcc.hse = Some(Hse {
            freq: Hertz(8_000_000),
            mode: HseMode::Oscillator,
        });
        config.rcc.pll1 = Some(Pll {
            source: PllSource::HSE,
            prediv: PllPreDiv::DIV4,
            mul: PllMul::MUL275,
            divp: Some(PllDiv::DIV1),
            divq: Some(PllDiv::DIV4),
            divr: None,
        });
        config.rcc.sys = Sysclk::PLL1_P;
        config.rcc.ahb_pre = AHBPrescaler::DIV2;
        config.rcc.apb1_pre = APBPrescaler::DIV2;
        config.rcc.apb2_pre = APBPrescaler::DIV2;
        config.rcc.apb3_pre = APBPrescaler::DIV2;
        config.rcc.apb4_pre = APBPrescaler::DIV2;
    }
    let p = embassy_stm32::init(config);

    // Watchdog. Pet it each loop iteration.
    // If main exits (HardError), petting stops and board resets after ~7s.
    let mut watchdog = IndependentWatchdog::new(p.IWDG1, 7_000_000);

    // LEDs — verify PE1 is yellow on your board
    let red = Output::new(p.PB14, Level::Low, Speed::Low);
    let yellow = Output::new(p.PE1, Level::Low, Speed::Low);
    let green = Output::new(p.PB0, Level::Low, Speed::Low);
    spawner.spawn(leds::led_task(red, yellow, green)).unwrap();

    // Ethernet — RMII pins on Nucleo-H723ZG
    let eth: net::Device = Ethernet::new(
        net::packet_queue(),
        p.ETH,
        Irqs,
        p.PA1,     // ref_clk
        p.PA7,     // crs_dv
        p.PC4,     // rxd0
        p.PC5,     // rxd1
        p.PG13,    // txd0
        p.PB13,    // txd1
        p.PG11,    // tx_en
        MAC_ADDR,
        p.ETH_SMA,
        p.PA2,     // mdio
        p.PC1,     // mdc
    );

    let mut rng = Rng::new(p.RNG, Irqs);
    let mut seed = [0u8; 8];
    rng.fill_bytes(&mut seed);

    let (stack, runner) = net::init_stack(eth, u64::from_le_bytes(seed));
    spawner.spawn(net::net_task(runner)).unwrap();

    // BME280
    let i2c = I2c::new(
        p.I2C2,
        p.PB10,
        p.PB11,
        p.DMA1_CH4,
        p.DMA1_CH5,
        Irqs,
        Default::default(),
    );
    let mut bme = Bme280::new(i2c);
    if bme.init().is_err() {
        leds::STATE.signal(BoardState::HardError);
        return;
    }

    info!("Waiting for DHCP...");
    // WaitingDhcp is the initial state — yellow already blinking
    stack.wait_config_up().await;
    let ip = stack.config_v4().unwrap().address.address().octets();
    info!("Network up: {}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3]);
    leds::STATE.signal(BoardState::Running);
    watchdog.unleash();

    loop {
        let m = bme.read().unwrap();
        info!(
            "Temp: {}.{} C | Pressure: {} Pa | Humidity: {}.{} %",
            m.temperature / 100,
            m.temperature % 100,
            m.pressure / 256,
            m.humidity / 1024,
            (m.humidity % 1024) * 100 / 1024,
        );

        if net::send_reading(stack, &m).await {
            watchdog.pet();
        } else {
            leds::STATE.signal(BoardState::SendFailed);
        }
        embassy_time::Timer::after_millis(500).await;
    }
}
