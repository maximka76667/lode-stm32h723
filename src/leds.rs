use embassy_futures::select::{Either, select};
use embassy_stm32::gpio::Output;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::Timer;

pub enum BoardState {
    WaitingDhcp,
    Running,
    SendFailed,
    HardError,
}

pub static STATE: Signal<CriticalSectionRawMutex, BoardState> = Signal::new();

#[embassy_executor::task]
pub async fn led_task(
    mut red: Output<'static>,
    mut yellow: Output<'static>,
    mut green: Output<'static>,
) -> ! {
    let mut state = BoardState::WaitingDhcp;

    loop {
        red.set_low();
        yellow.set_low();
        green.set_low();

        match state {
            BoardState::WaitingDhcp => loop {
                yellow.set_high();
                match select(Timer::after_millis(400), STATE.wait()).await {
                    Either::First(_) => {}
                    Either::Second(s) => {
                        state = s;
                        break;
                    }
                }
                yellow.set_low();
                match select(Timer::after_millis(400), STATE.wait()).await {
                    Either::First(_) => {}
                    Either::Second(s) => {
                        state = s;
                        break;
                    }
                }
            },

            BoardState::Running => {
                green.set_high();
                state = STATE.wait().await;
            }

            BoardState::SendFailed => {
                for _ in 0..3 {
                    red.set_high();
                    Timer::after_millis(150).await;
                    red.set_low();
                    Timer::after_millis(150).await;
                }
                state = BoardState::Running;
            }

            BoardState::HardError => {
                red.set_high();
                loop {
                    Timer::after_secs(3600).await;
                }
            }
        }
    }
}
