use crate::spi_device::CustomSpiDevice;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals;
use embassy_rp::spi::{Blocking, ClkPin, Config, CsPin, MisoPin, MosiPin, Spi};

// pub fn create_spi<'d>(
//     spi: peripherals::SPI0,
//     clk: peripherals::PIN_18,
//     mosi: peripherals::PIN_19,
//     miso: peripherals::PIN_16,
//     cs: peripherals::PIN_17,
pub fn create_spi<'d, ClkT, MosiT, MisoT, CsT>(
    spi: peripherals::SPI0,
    clk: ClkT,
    mosi: MosiT,
    miso: MisoT,
    cs: CsT,
) -> CustomSpiDevice<'d, peripherals::SPI0, Blocking>
where
    ClkT: ClkPin<peripherals::SPI0>,
    CsT: CsPin<peripherals::SPI0>,
    MosiT: MosiPin<peripherals::SPI0>,
    MisoT: MisoPin<peripherals::SPI0>,
{
    // Setup SPI
    let mut spi_config = Config::default();
    spi_config.frequency = 10_000_000;

    let rp_spi = Spi::new_blocking(spi, clk, mosi, miso, spi_config);
    let cs = Output::new(cs, Level::High);
    CustomSpiDevice::new(rp_spi, cs)
}

#[macro_export]
macro_rules! create_default_spi {
    ($p:expr) => {{
        spi::create_spi($p.SPI0, $p.PIN_18, $p.PIN_19, $p.PIN_16, $p.PIN_17)
    }};
}

#[macro_export]
macro_rules! create_default_spi_rpipico_w_waveshare_hat {
    ($p:expr) => {{
        spi::create_spi($p.SPI0, $p.PIN_2, $p.PIN_3, $p.PIN_4, $p.PIN_5)
    }};
}
