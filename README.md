# lode-stm32h723

Embedded firmware for the **NUCLEO-H723ZG** board. Reads temperature, humidity, and pressure from a BME280 sensor over I2C and POSTs the readings every 2 seconds to the [lode-api](https://github.com/maximka76667/lode-api-rust) backend over Ethernet via HTTPS.

Built with [Embassy](https://embassy.dev/) on Rust.

![lode-stm32](https://github.com/user-attachments/assets/a3be6681-b2a8-481c-9d3b-835d060538b2)

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
  lib.rs           — library root, declares modules
  bme280.rs        — BME280 driver (generic over embedded-hal I2C)
  net.rs           — Ethernet stack init and DHCP
  dns.rs           — DNS resolution with retry
  http.rs          — HTTPS POST via reqwless + embedded-tls
  leds.rs          — LED state machine
  fmt.rs           — defmt logging helpers
  bin/
    nucleo.rs      — main firmware binary
    bme280-test.rs — standalone sensor test
    hello.rs       — LED blink smoke test
```

## LED states

| LED             | State                                              |
| --------------- | -------------------------------------------------- |
| Yellow blinking | Waiting for DHCP                                   |
| Yellow off→on   | Resolving DNS (one pulse per attempt)              |
| Green solid     | Running, sending readings                          |
| Red blinks 3×   | Send failed, retrying                              |
| Red solid       | Hard error (BME280 not found), watchdog will reset |

## Watchdog

The IWDG watchdog is unleashed immediately at startup with a **10 seconds timeout**. This means a hang at any stage — DHCP, DNS, or sending — triggers a full reset. The watchdog is pet on each successful HTTPS POST, and also on each DNS resolution attempt during startup.

## Backend

Readings are sent to the production backend at `https://lode-api-rust.onrender.com/readings`.

The firmware resolves the hostname via DNS on startup (retrying every 5 seconds until successful), then opens a new TLS connection for each POST. Certificate verification is skipped (`TlsVerify::None`) — suitable for a trusted private endpoint.

To point at a different backend, edit `src/http.rs`:

```rust
pub const HOST: &str = "lode-api-rust.onrender.com";
pub const URL: &str  = "https://lode-api-rust.onrender.com/readings";
```

## Building and flashing

```bash
# Development — flash and stream defmt logs via RTT (requires probe-rs)
cargo run --bin nucleo

# Production — fully optimized, flash only, board runs standalone
cargo flash --bin nucleo --release --chip STM32H723ZGIx
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
