#![no_std]
#![no_main]

mod soft_timer;
mod spi;
mod spi_device;

use rp::{self, serial_type};

use static_cell::StaticCell;

use defmt::info;
use doggie_core::{
    core_create_tasks, core_run, Bsp, CanChannel, CanChannelReceiver, CanChannelSender, Core,
};
use embassy_rp::{
    bind_interrupts,
    gpio::{Level, Output},
    peripherals::SPI0,
    spi::Blocking,
};
#[cfg(feature = "mcp2515")]
use mcp2515::MCP2515;
#[cfg(feature = "mcp2518fd")]
use doggie_core::mcp2518fd::ExpMCP2518FD;

use soft_timer::SoftTimer;
use spi_device::CustomSpiDevice;
use {defmt_rtt as _, panic_probe as _};

use embassy_executor::Spawner;

rp::init_globals!();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Device initialization");
    let p = embassy_rp::init(Default::default());
    info!("Create serial");
    let serial = rp::create_serial!(p, spawner);

    info!("Setup SPI");
    // Setup SPI
    // default for MCP2515 :
    // let spi = create_default_spi!(p);
    // For Waveshare 2-CH-CAN-FD-HAT with expansion board:
    let spi = create_default_spi_rpipico_w_waveshare_hat!(p);
    info!("SPI init ok");

    // Create SoftTimer
    let delay = SoftTimer {};

    // Create the Bsp
    #[cfg(feature = "mcp2515")]
    let bsp = Bsp::new_with_mcp2515(spi, delay, serial);
    #[cfg(feature = "mcp2515")]
    info!("MCP2515 init ok");

    #[cfg(feature = "mcp2518fd")]
    let bsp = Bsp::new_with_mcp2518fd(spi, delay, serial);
    #[cfg(feature = "mcp2518fd")]
    info!("MCP2518FD init ok");

    // Create and run the Doggie core
    let core = Core::new(spawner, bsp);

    core_run!(core);
}

type SerialType = serial_type!();

#[cfg(feature = "mcp2515")]
type CanType = MCP2515<CustomSpiDevice<'static, SPI0, Blocking>>;
#[cfg(feature = "mcp2518fd")]
type CanType = ExpMCP2518FD<CustomSpiDevice<'static, SPI0, Blocking>, SoftTimer>;

core_create_tasks!(SerialType, CanType);
