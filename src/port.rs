//! Communication with device ports/interfaces.
//!
//! `lib.rs` deals with general device-level configuration, while this module deals with claiming
//! and communicating with individual ports/interfaces of a device.

use std::{cell::RefMut, fmt, time::Duration};

use crate::{
    prop::DeviceProps, ControlReq, Error, ErrorKind, Ftdi, Result, UsbHandle, REQ_READ, REQ_WRITE,
};

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
#[repr(u8)]
pub enum BitMode {
    Reset = 0x00,
    Bitbang = 0x01,
    Mpsse = 0x02,
    Syncbb = 0x04,
    Mcu = 0x08,
    Opto = 0x10,
    Cbus = 0x20,
    Syncff = 0x40,
}

/// A claimed port on an FTDI device.
///
/// Devices may have anywhere between 1 to 4 ports that can be individually claimed by different
/// applications. A port maps to a USB Interface.
pub struct Port {
    device: UsbHandle,
    timeout: Duration,
    /// Port/Interface index (0-based).
    index: u8,
    properties: &'static DeviceProps,
}

impl Port {
    pub(crate) fn open(parent: &Ftdi, index: u8) -> Result<Self> {
        let mut dev = parent.dev();
        dev.claim_interface(index).map_err(Error::usb)?;
        drop(dev);

        let mut this = Self {
            device: parent.device.clone(),
            timeout: parent.timeout,
            index,
            properties: parent.properties,
        };

        this.set_bitmode(BitMode::Reset)?;

        Ok(this)
    }

    fn dev(&self) -> RefMut<'_, rusb::DeviceHandle<rusb::GlobalContext>> {
        self.device.borrow_mut()
    }

    fn read_control<'b>(&self, request: ControlReq, value: u16, buf: &'b mut [u8]) -> Result<()> {
        let n = self
            .dev()
            .read_control(
                REQ_READ,
                request as u8,
                value,
                u16::from(self.index) + 1, // bInterfaceNumber + 1
                buf,
                self.timeout,
            )
            .map_err(Error::usb)?;
        if n != buf.len() {
            log::error!("read_control: read {} bytes, expected {}", n, buf.len());
            return Err(Error::from_kind(ErrorKind::Other));
        }
        Ok(())
    }

    fn write_control(&self, request: ControlReq, value: u16, buf: &[u8]) -> Result<()> {
        let n = self
            .dev()
            .write_control(
                REQ_WRITE,
                request as u8,
                value,
                u16::from(self.index) + 1, // bInterfaceNumber + 1
                buf,
                self.timeout,
            )
            .map_err(Error::usb)?;
        if n != buf.len() {
            log::error!("write_control: wrote {} bytes, expected {}", n, buf.len());
            return Err(Error::from_kind(ErrorKind::Other));
        }

        Ok(())
    }

    pub fn set_bitmode(&mut self, mode: BitMode) -> Result<()> {
        self.write_control(ControlReq::SetBitmode, (mode as u16) << 8 | 0x00, &[])?;
        Ok(())
    }

    /// Polls the current status of the lower 8 I/O pins.
    ///
    /// **Note**: This only returns the low 8 bits. If the port has more than 8 data pins, the upper
    /// pins cannot be fetched with this function.
    pub fn read_pins(&self) -> Result<u8> {
        let mut buf = [0; 1];
        self.read_control(ControlReq::ReadPins, 0, &mut buf)?;

        Ok(buf[0])
    }
}

impl Drop for Port {
    fn drop(&mut self) {
        self.dev().release_interface(self.index).ok();
    }
}

impl fmt::Debug for Port {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Port")
            .field("index", &self.index)
            .field("timeout", &self.timeout)
            .finish()
    }
}
