use crate::can::{CanBitrates, CanDevice};
use defmt::{error, info};
use embedded_can::{ExtendedId, Id};
use embedded_hal::{delay::DelayNs, spi::SpiDevice};
use embedded_io_async::{Read, Write};

use crate::bsp::Bsp;

use embedded_can::{blocking::Can, Frame};

use mcp2518fd::{
    memory::controller::{
        configuration::OperationMode,
        fifo::{FifoNumber, PayloadSize, HIGHEST_FIFO_PRIORITY},
        filter::FilterNumber,
    },
    message::rx::RxMessage,
    message::tx::TxMessage,
    settings::{
        BitTimeConfiguration, DataBitTimeConfiguration, FifoConfiguration, FifoMode,
        FilterConfiguration, FilterMatchMode, IoConfiguration, NominalBitTimeConfiguration,
        OscillatorConfiguration, RxFifoConfiguration, Settings, TxEventFifoConfiguration,
        TxQueueConfiguration,
    },
    spi::MCP2518FD,
};

// We need to store a copy of the delay object for the CanDevice::listen_only()
// We also need to store the filter settings so that we can construct the FilterConfiguration
// from two calls of two separate methods (set_filter, set_mask).
pub struct ExpMCP2518FD<SPI, DE> {
    drv: MCP2518FD<SPI>,
    dly: DE,
    filter_config_filter: Id,
    filter_config_mask: Id,
}

impl<SPI: SpiDevice, DE> ExpMCP2518FD<SPI, DE> {
    fn set_filter_and_mask(&mut self) {
        let fid_raw: u32 = match self.filter_config_filter {
            Id::Standard(id) => id.as_raw().into(),
            Id::Extended(id) => id.as_raw(),
        };
        let mid_raw: u32 = match self.filter_config_mask {
            Id::Standard(id) => id.as_raw().into(),
            Id::Extended(id) => id.as_raw(),
        };

        info!("Setting filter 0x{:x} and mask 0x{:x}.", fid_raw, mid_raw);

        self.drv
            .configure_filter(
                FilterNumber::Filter0,
                Some(FilterConfiguration {
                    buffer_pointer: FifoNumber::Fifo1,
                    mode: FilterMatchMode::Both,
                    filter_bits: self.filter_config_filter,
                    mask_bits: self.filter_config_mask,
                }),
            )
            .expect("Failed to configure Filter 0 for FIFO 1.");
    }
}

#[derive(Debug)]
pub struct MyError(mcp2518fd::Error);

/// Wrapper for the MCP2518FD different Tx and Rx Messages.
pub enum MyFrame {
    Tx(TxMessage),
    Rx(RxMessage),
}

impl embedded_can::Error for MyError {
    fn kind(&self) -> embedded_can::ErrorKind {
        match self.0 {
            mcp2518fd::Error::CrcMismatch => embedded_can::ErrorKind::Crc,
            _ => embedded_can::ErrorKind::Other,
        }
    }
}

impl Frame for MyFrame {
    fn new(id: impl Into<Id>, data: &[u8]) -> Option<Self> {
        let txm = TxMessage::new_2_0(id, data);
        match txm {
            Some(x) => Some(MyFrame::Tx(x)),
            None => None,
        }
    }

    fn new_remote(id: impl Into<Id>, dlc: usize) -> Option<Self> {
        let txm = TxMessage::new_remote(id, dlc as u8);
        match txm {
            Some(x) => Some(MyFrame::Tx(x)),
            None => None,
        }
    }

    fn is_extended(&self) -> bool {
        match self {
            MyFrame::Tx(msg) => return msg.header().eid() > 0,
            MyFrame::Rx(msg) => return msg.header().eid() > 0,
        }
    }

    fn is_remote_frame(&self) -> bool {
        match self {
            MyFrame::Tx(msg) => msg.header().rtr(),
            MyFrame::Rx(msg) => msg.header().rtr(),
        }
    }

    fn id(&self) -> Id {
        match self {
            MyFrame::Tx(msg) => msg.id(),
            MyFrame::Rx(msg) => msg.id(),
        }
    }

    fn dlc(&self) -> usize {
        match self {
            MyFrame::Tx(msg) => msg.header().dlc() as usize,
            MyFrame::Rx(msg) => msg.header().dlc() as usize,
        }
    }

    fn data(&self) -> &[u8] {
        match self {
            MyFrame::Tx(msg) => msg.data(),
            MyFrame::Rx(msg) => msg.data(),
        }
    }
}

impl<SPI: SpiDevice, DELAY: DelayNs> Can for ExpMCP2518FD<SPI, DELAY> {
    type Frame = MyFrame;
    type Error = MyError;

    fn transmit(&mut self, frame: &Self::Frame) -> Result<(), Self::Error> {
        let Some(txm) = TxMessage::new_2_0(frame.id(), frame.data()) else {
            error!("Error creating new message.");
            let me: MyError = MyError(mcp2518fd::Error::Other);
            return Result::Err(me);
        };

        match self.drv.tx_queue_transmit_message(&txm) {
            Ok(x) => {
                info!("Tx queue transmit message successful.");
                return Ok(x);
            }
            Err(e) => {
                error!("Tx queue transmit message failed: {:?}", e);
                return Result::Err(MyError(e));
            }
        }
    }

    fn receive(&mut self) -> Result<Self::Frame, Self::Error> {
        match self.drv.rx_fifo_get_next(FifoNumber::Fifo1) {
            Ok(Some(rxmsg)) => Ok(MyFrame::Rx(rxmsg)),
            Ok(None) => Err(MyError(mcp2518fd::Error::Other)),
            Err(e) => panic!("Error reading from FIFO: {:?}", e),
        }
    }
}

impl<SPI: SpiDevice, DELAY: DelayNs> CanDevice for ExpMCP2518FD<SPI, DELAY> {
    fn set_bitrate(&mut self, bitrate: CanBitrates) {
        match bitrate {
            CanBitrates::Kbps100 => {
                info!("Setting bitrate to 100 kbit.");
                self.drv
                    .configure_bit_timing(BitTimeConfiguration::new(
                        NominalBitTimeConfiguration::RATE_100_KBIT,
                        DataBitTimeConfiguration::RATE_2_MBIT,
                    ))
                    .expect("Cannot set bitrate to 100 kbit.");
            }
            CanBitrates::Kbps125 => {
                info!("Setting bitrate to 125 kbit.");
                self.drv
                    .configure_bit_timing(BitTimeConfiguration::new(
                        NominalBitTimeConfiguration::RATE_125_KBIT,
                        DataBitTimeConfiguration::RATE_2_MBIT,
                    ))
                    .expect("Cannot set bitrate to 125 kbit.");
            }
            CanBitrates::Kbps250 => {
                info!("Setting bitrate to 250 kbit.");
                self.drv
                    .configure_bit_timing(BitTimeConfiguration::new(
                        NominalBitTimeConfiguration::RATE_250_KBIT,
                        DataBitTimeConfiguration::RATE_2_MBIT,
                    ))
                    .expect("Cannot set bitrate to 250 kbit.");
            }
            CanBitrates::Kbps500 => {
                info!("Setting bitrate to 500 kbit.");
                self.drv
                    .configure_bit_timing(BitTimeConfiguration::new(
                        NominalBitTimeConfiguration::RATE_500_KBIT,
                        DataBitTimeConfiguration::RATE_2_MBIT,
                    ))
                    .expect("Cannot set bitrate to 500 kbit.");
            }
            CanBitrates::Kbps1000 => {
                info!("Setting bitrate to 1 Mbit.");
                self.drv
                    .configure_bit_timing(BitTimeConfiguration::new(
                        NominalBitTimeConfiguration::RATE_1_MBIT,
                        DataBitTimeConfiguration::RATE_2_MBIT,
                    ))
                    .expect("Cannot set bitrate to 1 Mbit.");
            }
            _ => {
                error!("Requested speed that does not match preset values in MCP2518FD.")
            }
        }
    }

    fn set_filter(&mut self, id: Id) {
        self.filter_config_filter = id;
        self.set_filter_and_mask();
    }

    fn set_mask(&mut self, id: Id) {
        self.filter_config_mask = id;
        self.set_filter_and_mask();
    }

    fn open(&mut self) {
        info!("Set operation mode to CAN 2.0.");
        self.drv
            .set_op_mode(OperationMode::NormalCan2, &mut self.dly)
            .expect("Failed to change chip operating mode.");
    }

    fn close(&mut self) {
        info!("Called close, does nothing.");
    }

    fn listen_only(&mut self) {
        info!("Set operation mode to listen only.");
        self.drv
            .set_op_mode(OperationMode::ListenOnly, &mut self.dly)
            .expect("Failed to change chip operating mode.");
    }
}

impl<SPI, SERIAL, DELAY> Bsp<ExpMCP2518FD<SPI, DELAY>, SERIAL>
where
    SPI: SpiDevice,
    SERIAL: Read + Write,
    DELAY: DelayNs + Copy,
{
    pub fn new_with_mcp2518fd(spi: SPI, mut delay: DELAY, serial: SERIAL) -> Self {
        // Create a new ExpMCP2518FD module:
        let mut tc: ExpMCP2518FD<SPI, DELAY> = ExpMCP2518FD {
            drv: MCP2518FD::new(spi),
            dly: delay,
            filter_config_filter: Id::Extended(ExtendedId::ZERO),
            filter_config_mask: Id::Extended(ExtendedId::ZERO),
        };

        // Reset the module:
        tc.drv.reset().expect("Failed to reset MCP2518FD.");

        // Configure the chip with some reasonable settings
        tc.drv
            .configure(
                Settings {
                    // Standard for 40MHz XTAL
                    oscillator: OscillatorConfiguration::default(),
                    // Use default values for IOCON register
                    io_configuration: IoConfiguration::new(),
                    // Configure the bit timings (assumes a 40MHz input clock)
                    bit_time_configuration: BitTimeConfiguration::new(
                        NominalBitTimeConfiguration::RATE_500_KBIT,
                        DataBitTimeConfiguration::RATE_2_MBIT,
                    ),
                    // Store the last 12 transmitted messages in the TEF with timestamps
                    tx_event_fifo: Some(TxEventFifoConfiguration::new(12).with_timestamps(false)),
                    // Configure TXQ to have priority over all other FIFOs, and to
                    // hold up to 8 messages with a max payload size of 32 bytes
                    tx_queue: Some(TxQueueConfiguration::new(
                        HIGHEST_FIFO_PRIORITY,
                        8,
                        PayloadSize::Bytes32,
                    )),
                    // Enable the Time Based Counter (required for timestamps to be
                    // recorded as non-zero)
                    enable_time_based_counter: true,
                    // Do not filter by any data bits
                    data_bits_to_match: None,
                    // Do not interrupt on CAN bus errors
                    enable_can_error_interrupts: false,
                    // Do not interrupt on SPI comms errors
                    enable_spi_error_interrupt: false,
                    // Do not interrupt on RAM ECC errors
                    enable_ecc_error_interrupt: false,
                },
                &mut delay,
            )
            .expect("Failed to configure MCP2518FD.");

        // Configure FIFO 1 as an RX FIFO to hold up to 16 messages with a max
        // payload size of 8 bytes
        tc.drv
            .configure_fifo(
                FifoNumber::Fifo1,
                FifoConfiguration {
                    fifo_size: 16,
                    payload_size: PayloadSize::Bytes8,
                    mode: FifoMode::Receive(
                        RxFifoConfiguration::new().with_message_timestamps(true),
                    ),
                },
            )
            .expect("Failed to configure FIFO 1 as RX.");

        // Configure Filter 0 to accept all frame types (Standard or Extended),
        // with any message ID (mask is all 0s)
        tc.drv
            .configure_filter(
                FilterNumber::Filter0,
                Some(FilterConfiguration {
                    buffer_pointer: FifoNumber::Fifo1,
                    mode: FilterMatchMode::Both,
                    filter_bits: Id::Extended(ExtendedId::ZERO),
                    mask_bits: Id::Extended(ExtendedId::ZERO),
                }),
            )
            .expect("Failed to configure Filter 0 for FIFO 1.");

        Bsp::new(tc, serial)
    }
}
