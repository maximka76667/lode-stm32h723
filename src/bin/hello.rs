#![no_std]
#![no_main]

#[path = "../fmt.rs"]
mod fmt;

#[cfg(not(feature = "defmt"))]
use panic_halt as _;
#[cfg(feature = "defmt")]
use {defmt_rtt as _, panic_probe as _};

use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_time::{Duration, Timer};
use fmt::info;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    let mut red_led = Output::new(p.PB14, Level::High, Speed::Low);
    let mut green_led = Output::new(p.PB0, Level::High, Speed::Low);

    loop {
        info!("Hello, World!");
        red_led.set_high();
        green_led.set_low();
        Timer::after(Duration::from_millis(500)).await;
        red_led.set_low();
        green_led.set_high();
        Timer::after(Duration::from_millis(500)).await;
    }
}
