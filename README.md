# lode-stm32h723

Embedded firmware for the **NUCLEO-H723ZG** board. Reads temperature, humidity, and pressure from a BME280 sensor over I2C and POSTs the readings every 500ms to the [lode-api](https://github.com/maximka76667/lode-api-rust) backend over Ethernet.

Built with [Embassy](https://embassy.dev/) on Rust.

## Hardware

| Component | Details                      |
| --------- | ---------------------------- |
| Board     | STM32 NUCLEO-H723ZG          |
| Sensor    | BME280 (I2C, address `0x76`) |
| PHY       | LAN8742A (onboard RMII)      |

### Wiring

**BME280 → Nucleo (I2C2)**

| BME280 | Nucleo pin |
| ------ | ---------- |
| SDA    | PB11       |
| SCL    | PB10       |
| VCC    | 3.3V       |
| GND    | GND        |

**Ethernet** — RJ45 connects directly to the onboard LAN8742A, no extra wiring needed.

## Project structure

```
src/
  lib.rs          — library root, declares modules
  bme280.rs       — BME280 driver (generic over embedded-hal I2C)
  net.rs          — Ethernet setup, DHCP, HTTP POST
  leds.rs         — LED state machine
  fmt.rs          — defmt logging helpers
  bin/
    nucleo.rs     — main firmware binary
    bme280-test.rs — standalone sensor test
    hello.rs      — LED blink smoke test
```

## LED states

| LED             | State                                              |
| --------------- | -------------------------------------------------- |
| Yellow blinking | Waiting for DHCP                                   |
| Green solid     | Running, sending readings                          |
| Red blinks 3×   | Send failed, retrying                              |
| Red solid       | Hard error (BME280 not found), watchdog will reset |

## Watchdog

The IWDG watchdog is unleashed after DHCP and BME280 init with a **7 second timeout**. It is only pet on a successful HTTP POST. If the backend is unreachable for 7 seconds straight the board resets and reconnects from scratch.

## Configuration

Edit `src/net.rs` to point at your backend:

```rust
pub const SERVER_ADDR: [u8; 4] = [192, 168, 1, 136];
pub const SERVER_HOST: &str = "192.168.1.136";
pub const SERVER_PORT: u16 = 3111;
```

## Building and flashing

```bash
# Flash and run with RTT logging (requires probe-rs)
cargo run --bin nucleo

# Flash only (standalone, no debugger)
probe-rs download --chip STM32H723ZGIx target/thumbv7em-none-eabi/debug/nucleo
```

For standalone operation, power the board via the mini USB port (CN2) from any 5V USB charger or power bank.

## API format

Each reading is sent as an HTTP POST to `/readings`:

```json
{
  "temperature_c": 23.15,
  "humidity_pct": 50.12,
  "pressure_hpa": 1013.25
}
```
