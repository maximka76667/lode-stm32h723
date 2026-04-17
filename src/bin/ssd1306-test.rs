#![no_std]
#![no_main]

#[path = "../fmt.rs"]
mod fmt;
use fmt::info;

use embassy_executor::Spawner;
use embassy_stm32::i2c::I2c;
use embassy_time::Timer;
use lode_stm32h723::ssd1306::Ssd1306;

#[cfg(not(feature = "defmt"))]
use panic_halt as _;
#[cfg(feature = "defmt")]
use {defmt_rtt as _, panic_probe as _};

// Draws a 1-pixel border around the full 128x64 display.
fn draw_border(display: &mut Ssd1306<impl embedded_hal::i2c::I2c>) {
    for x in 0..128 {
        display.set_pixel(x, 0, true);
        display.set_pixel(x, 63, true);
    }
    for y in 0..64 {
        display.set_pixel(0, y, true);
        display.set_pixel(127, y, true);
    }
}

// Draws diagonal lines from all four corners to the centre so you can
// immediately see if the display is rotated or mirrored.
fn draw_diagonals(display: &mut Ssd1306<impl embedded_hal::i2c::I2c>) {
    for i in 0..64usize {
        let x = i * 127 / 63;
        display.set_pixel(x, i, true);
        display.set_pixel(127 - x, i, true);
    }
}

// Solid 6x6 square in the top-left corner — the "this side up" marker.
fn draw_origin_marker(display: &mut Ssd1306<impl embedded_hal::i2c::I2c>) {
    for x in 2..8 {
        for y in 2..8 {
            display.set_pixel(x, y, true);
        }
    }
}

// Checkerboard that covers the entire framebuffer — useful for checking every pixel.
fn draw_checkerboard(display: &mut Ssd1306<impl embedded_hal::i2c::I2c>) {
    for y in 0..64usize {
        for x in 0..128usize {
            display.set_pixel(x, y, (x + y) % 2 == 0);
        }
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    // SCL → PD1, SDA → PG0.
    // If this doesn't compile, check which I2C peripheral those pins
    // belong to on your board and swap p.I2C4 accordingly.
    let i2c = I2c::new_blocking(p.I2C4, p.PB8, p.PB9, Default::default());

    let mut display = Ssd1306::new(i2c);
    display.init().unwrap();
    info!("SSD1306 init OK");

    // Frame A: border + diagonals + origin marker
    display.clear();
    draw_border(&mut display);
    draw_diagonals(&mut display);
    draw_origin_marker(&mut display);
    display.flush().unwrap();
    info!("frame A: border + diagonals + origin marker");

    Timer::after_secs(3).await;

    // Frame B: checkerboard — every pixel exercised
    display.clear();
    draw_checkerboard(&mut display);
    display.flush().unwrap();
    info!("frame B: checkerboard");

    Timer::after_secs(3).await;

    // Frame C: blank — confirms clear() works
    display.clear();
    display.flush().unwrap();
    info!("frame C: blank");

    Timer::after_secs(1).await;

    // Blink the origin marker so you can tell the firmware is still running
    let mut on = true;
    loop {
        display.clear();
        draw_border(&mut display);
        draw_origin_marker(&mut display);
        if on {
            draw_diagonals(&mut display);
        }
        display.flush().unwrap();
        on = !on;
        Timer::after_millis(500).await;
    }
}
