use embassy_stm32::{
    mode,
    usart::{self, Uart},
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use ld2410c::{Ld2410c, TargetData, UartReader};

pub static PRESENCE: Signal<CriticalSectionRawMutex, TargetData> = Signal::new();

struct Ld2410cUart<'d>(Uart<'d, mode::Async>);

impl UartReader for Ld2410cUart<'_> {
    type Error = usart::Error;

    async fn read_until_idle(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.0.read_until_idle(buf).await
    }
}

#[embassy_executor::task]
pub async fn presence_task(uart: Uart<'static, mode::Async>) -> ! {
    let mut driver = Ld2410c::new(Ld2410cUart(uart));
    let mut buf = [0u8; 128];

    loop {
        if let Ok(Some(data)) = driver.read_frame(&mut buf).await {
            PRESENCE.signal(data);
        }
    }
}
