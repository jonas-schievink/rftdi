use crate::{bitmode, ControlReq, Port, Result};
use bitflags::bitflags;

bitflags! {
    pub struct ModemStatus: u16 {
        /// Clear to send.
        const CTS = 1 << 4;
        /// Data set ready.
        const DSR = 1 << 5;
        /// Ring indicator.
        const RI = 1 << 6;
        /// Data carrier detect.
        const DCD = 1 << 7;
        /// Data ready.
        const DR = 1 << 8;
        /// Overrun error.
        const OE = 1 << 9;
        /// Parity error.
        const PE = 1 << 10;
        /// Framing error.
        const FE = 1 << 11;
        /// Break interrupt.
        const BI = 1 << 12;
        /// Transmitter holding register empty.
        const THRE = 1 << 13;
        /// Transmitter empty.
        const TEMT = 1 << 14;
        /// Error in RECV FIFO.
        const ERR = 1 << 15;
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum FlowControl {
    Disabled,
    RtsCts,
    DtrDsr,
    XonXoff,
}

impl Default for FlowControl {
    fn default() -> Self {
        FlowControl::Disabled
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Parity {
    None = 0x00,
    Odd = 0x01,
    Even = 0x02,
    Mark = 0x03,
    Space = 0x04,
}

impl Default for Parity {
    fn default() -> Self {
        Parity::None
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum StopBits {
    Stop1 = 0x00,
    Stop15 = 0x01,
    Stop2 = 0x02,
}

impl Default for StopBits {
    fn default() -> Self {
        StopBits::Stop1
    }
}

const MODEM_CTRL_SET_DTR_HIGH: u16 = 0x0101;
const MODEM_CTRL_SET_DTR_LOW: u16 = 0x0100;
const MODEM_CTRL_SET_RTS_HIGH: u16 = 0x0202;
const MODEM_CTRL_SET_RTS_LOW: u16 = 0x0200;

/// Functionality available when in serial mode.
impl Port<bitmode::Serial> {
    pub fn poll_modem_status(&self) -> Result<ModemStatus> {
        let mut buf = [0; 2];
        self.read_control(ControlReq::PollModemStatus, 0, &mut buf)?;
        Ok(ModemStatus::from_bits_truncate(u16::from_le_bytes(buf)))
    }

    /// Sets or clears the Data Terminal Ready (DTR) bit.
    ///
    /// Note that the DTR output pin is inverted (DTR#), so the pin state will be the opposite of
    /// `dtr`.
    pub fn set_dtr(&mut self, dtr: bool) -> Result<()> {
        let value = if dtr {
            MODEM_CTRL_SET_DTR_HIGH
        } else {
            MODEM_CTRL_SET_DTR_LOW
        };
        self.write_control(ControlReq::SetModemCtrl, value, &[])
    }

    /// Sets or clears the Request To Send (RTS) bit.
    ///
    /// Note that the RTS output pin is inverted (RTS#), so the pin state will be the opposite of
    /// `rts`.
    pub fn set_rts(&mut self, rts: bool) -> Result<()> {
        let value = if rts {
            MODEM_CTRL_SET_RTS_HIGH
        } else {
            MODEM_CTRL_SET_RTS_LOW
        };
        self.write_control(ControlReq::SetModemCtrl, value, &[])
    }

    pub fn set_flow_control(&mut self, flow: FlowControl) -> Result<()> {
        let value = match flow {
            FlowControl::Disabled => 0x0000,
            FlowControl::RtsCts => 0x0100,
            FlowControl::DtrDsr => 0x0200,
            FlowControl::XonXoff => 0x0400,
        };

        self.write_control(ControlReq::SetFlowCtrl, value, &[])
    }

    pub fn set_serial_config(
        &mut self,
        parity: Parity,
        stop: StopBits,
        break_condition: bool,
    ) -> Result<()> {
        // FIXME: Apparently this can also set the word size?

        let parity = parity as u16;
        let stop = stop as u16;
        let break_condition = break_condition as u16;
        let value = parity << 8 | stop << 11 | break_condition << 14;

        self.write_control(ControlReq::SetData, value, &[])
    }

    pub fn set_event_char(&mut self, event: Option<u8>) -> Result<()> {
        let value = match event {
            Some(b) => 0x100 | u16::from(b),
            None => 0,
        };

        self.write_control(ControlReq::SetEventChar, value, &[])
    }

    pub fn set_error_char(&mut self, error: Option<u8>) -> Result<()> {
        let value = match error {
            Some(b) => 0x100 | u16::from(b),
            None => 0,
        };

        self.write_control(ControlReq::SetErrorChar, value, &[])
    }

    pub fn read_latency_timer(&self) -> Result<u8> {
        let mut buf = [0; 1];
        self.read_control(ControlReq::GetLatencyTimer, 0, &mut buf)?;
        Ok(buf[0])
    }

    pub fn set_latency_timer(&mut self, time: u8) -> Result<()> {
        assert!(12 <= time);
        self.write_control(ControlReq::SetLatencyTimer, time.into(), &[])
    }
}
