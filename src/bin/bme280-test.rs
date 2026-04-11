#![no_std]
#![no_main]

#[path = "../fmt.rs"]
mod fmt;
use embassy_time::Timer;
use fmt::info;

use embassy_executor::Spawner;
use embassy_stm32::{
    bind_interrupts, dma,
    i2c::{self, I2c},
    peripherals,
};
use lode_stm32h723::bme280::Bme280;

#[cfg(not(feature = "defmt"))]
use panic_halt as _;
#[cfg(feature = "defmt")]
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    I2C2_EV => i2c::EventInterruptHandler<peripherals::I2C2>;
    I2C2_ER => i2c::ErrorInterruptHandler<peripherals::I2C2>;
    DMA1_STREAM4 => dma::InterruptHandler<peripherals::DMA1_CH4>;
    DMA1_STREAM5 => dma::InterruptHandler<peripherals::DMA1_CH5>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

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
    bme.init().unwrap();

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
        Timer::after_millis(500).await;
    }
}
