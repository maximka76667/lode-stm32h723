use embedded_hal::i2c::I2c;

use crate::font::FONT_5X7;

const DEVICE_ADDRESS: u8 = 0x3C;

#[derive(Debug)]
pub enum Error<E> {
    I2c(E),
}

pub struct Ssd1306<I2C> {
    i2c: I2C,
    framebuffer: [u8; 1024],
}

impl<I2C: I2c> Ssd1306<I2C> {
    pub fn new(i2c: I2C) -> Self {
        Self {
            i2c,
            framebuffer: [0u8; 1024],
        }
    }

    pub fn init(&mut self) -> Result<(), Error<I2C::Error>> {
        // Turn off
        self.turn_off();

        // Set MUX Ratio
        self.cmd(0xA8)?;
        self.cmd(0x3F)?;

        // Set display offset
        self.cmd(0xD3)?;
        self.cmd(0x00)?;

        // Set display start line
        self.cmd(0x40)?;

        // Set Segment re-map (horizontal direction)
        self.cmd(0xA1)?;

        // Set COM output scan direction (vertical direction)
        self.cmd(0xC8)?;

        // Set COM Pins hardware configuration
        self.cmd(0xDA)?;
        self.cmd(0x12)?;

        // Set constrast control
        self.cmd(0x81)?;
        self.cmd(0x7F)?;

        // Disable entire display on
        self.cmd(0xA4)?;

        // Set normal display
        self.cmd(0xA6)?;

        // Set osc frequency
        self.cmd(0xD5)?;
        self.cmd(0x80)?;

        // Enable charge pump regulator
        self.cmd(0x8D)?;
        self.cmd(0x14)?;

        // Horizontal addressing mode — required for the framebuffer flush to work correctly
        self.cmd(0x20)?;
        self.cmd(0x00)?;

        // Display on
        self.cmd(0xAF)?;

        Ok(())
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, on: bool) -> Option<()> {
        if x >= 128 || y >= 64 {
            return None;
        }

        let byte_index = (y / 8) * 128 + x;
        let bit_index = y % 8;

        if on {
            self.framebuffer[byte_index] |= 1 << bit_index;
        } else {
            self.framebuffer[byte_index] &= !(1 << bit_index);
        }

        Some(())
    }

    pub fn flush(&mut self) -> Result<(), Error<I2C::Error>> {
        // Set columns
        self.cmd(0x21)?;
        self.cmd(0x00)?;
        self.cmd(0x7F)?;

        // Set pages
        self.cmd(0x22)?;
        self.cmd(0x00)?;
        self.cmd(0x07)?;

        // Creating a buffer one byte bigger than the actual data to include control byte for data (0x40)
        let mut payload = [0u8; 1025];

        payload[0] = 0x40;
        payload[1..].copy_from_slice(&self.framebuffer);
        self.i2c
            .write(DEVICE_ADDRESS, &payload)
            .map_err(Error::I2c)?;

        Ok(())
    }

    fn cmd(&mut self, command: u8) -> Result<(), Error<I2C::Error>> {
        self.i2c
            .write(DEVICE_ADDRESS, &[0x00, command])
            .map_err(Error::I2c)
    }

    pub fn clear(&mut self) {
        self.framebuffer.fill(0);
    }

    pub fn draw_char(&mut self, x: usize, y: usize, c: u8) {
        if c < 0x20 || c > 0x7E || x + 5 > 128 || y + 7 > 64 {
            return;
        }
        let glyph = &FONT_5X7[(c - 0x20) as usize];
        for col in 0..5usize {
            for row in 0..7usize {
                if glyph[col] & (1 << row) != 0 {
                    self.set_pixel(x + col, y + row, true);
                }
            }
        }
    }

    pub fn draw_str(&mut self, x: usize, y: usize, s: &str) {
        let mut cx = x;
        for byte in s.bytes() {
            if cx + 5 > 128 {
                break;
            }
            self.draw_char(cx, y, byte);
            cx += 6;
        }
    }

    fn turn_off(&mut self) {
        let _ = self.cmd(0xAE);
    }
}
