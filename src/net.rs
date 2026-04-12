use embassy_net::{Runner, Stack, StackResources};
use embassy_stm32::eth::{Ethernet, GenericPhy, PacketQueue, Sma};
use embassy_stm32::peripherals;
use static_cell::StaticCell;

pub type Device = Ethernet<'static, peripherals::ETH, GenericPhy<Sma<'static, peripherals::ETH_SMA>>>;

static PACKETS: StaticCell<PacketQueue<4, 4>> = StaticCell::new();
// DNS needs one extra socket slot (4 instead of 3).
static RESOURCES: StaticCell<StackResources<4>> = StaticCell::new();

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
