//! Communication with device ports/interfaces.
//!
//! `lib.rs` deals with general device-level configuration, while this module deals with claiming
//! and communicating with individual ports/interfaces of a device.

use std::any::type_name;
use std::cell::RefMut;
use std::fmt;
use std::marker::PhantomData;
use std::time::Duration;

use bitflags::bitflags;

use crate::bitmode::{self, AnyBitMode, BitMode};
use crate::prop::DeviceProps;
use crate::{ControlReq, Error, Ftdi, Result, UsbHandle, REQ_READ, REQ_WRITE};

bitflags! {
    pub struct ResetFlags: u16 {
        const PURGE_RX = 1;
        const PURGE_TX = 2;
        const PURGE_RX_TX = 1 | 2;
    }
}

// FIXME: Hack needed since you can't move out of types that impl `Drop`.
struct ReleaseOnDrop {
    /// Port/Interface index (0-based).
    index: u8,
    device: UsbHandle,
}

impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        self.device.borrow_mut().release_interface(self.index).ok();
    }
}

/// A claimed port on an FTDI device.
///
/// Devices may have anywhere between 1 to 4 ports that can be individually claimed by different
/// applications. A port maps to a USB Interface.
pub struct Port<M: AnyBitMode = bitmode::Serial> {
    device: ReleaseOnDrop,
    timeout: Duration,
    /// Bulk IN endpoint address.
    ep_in: u8,
    /// Bulk OUT endpoint address.
    ep_out: u8,
    properties: &'static DeviceProps,
    _p: PhantomData<M>,
}

impl Port {
    pub(crate) fn open(parent: &Ftdi, index: u8, ep_in: u8, ep_out: u8) -> Result<Self> {
        let mut dev = parent.dev();
        dev.claim_interface(index).map_err(Error::usb)?;
        drop(dev);

        let mut this = Self {
            device: ReleaseOnDrop {
                device: parent.device.clone(),
                index,
            },
            timeout: parent.timeout,
            ep_in,
            ep_out,
            properties: parent.properties,
            _p: PhantomData,
        };

        this.reset(ResetFlags::PURGE_RX_TX)?;
        // FIXME: `reset` does not reset most state, we should probably configure some reasonable
        // defaults here.
        this.set_bitmode(BitMode::Serial)?;

        Ok(this)
    }
}

impl<M: AnyBitMode> Port<M> {
    pub(crate) fn dev(&self) -> RefMut<'_, rusb::DeviceHandle<rusb::GlobalContext>> {
        self.device.device.borrow_mut()
    }

    pub(crate) fn read_control<'b>(
        &self,
        request: ControlReq,
        value: u16,
        buf: &'b mut [u8],
    ) -> Result<()> {
        let n = self
            .dev()
            .read_control(
                REQ_READ,
                request as u8,
                value,
                u16::from(self.device.index) + 1, // bInterfaceNumber + 1
                buf,
                self.timeout,
            )
            .map_err(Error::usb)?;
        if n != buf.len() {
            return Err(Error::other(format!(
                "read {} bytes, expected {}",
                n,
                buf.len()
            )));
        }
        Ok(())
    }

    pub(crate) fn write_control(&self, request: ControlReq, value: u16, buf: &[u8]) -> Result<()> {
        let n = self
            .dev()
            .write_control(
                REQ_WRITE,
                request as u8,
                value,
                u16::from(self.device.index) + 1, // bInterfaceNumber + 1
                buf,
                self.timeout,
            )
            .map_err(Error::usb)?;
        if n != buf.len() {
            return Err(Error::other(format!(
                "wrote {} bytes, expected {}",
                n,
                buf.len()
            )));
        }

        Ok(())
    }

    fn set_bitmode(&mut self, mode: BitMode) -> Result<()> {
        self.write_control(ControlReq::SetBitmode, (mode as u16) << 8 | 0x00, &[])?;
        Ok(())
    }

    /// Switches the port to mode `T`.
    ///
    /// This consumes the port and returns a new instance with mode parameter `T`.
    pub fn into_mode<T: AnyBitMode>(mut self) -> Result<Port<T>> {
        self.set_bitmode(T::MODE)?;
        Ok(Port {
            device: self.device,
            timeout: self.timeout,
            properties: self.properties,
            ep_in: self.ep_in,
            ep_out: self.ep_out,
            _p: PhantomData,
        })
    }

    /// Returns this Port's 0-based index.
    pub fn index(&self) -> u8 {
        self.device.index
    }

    /// Returns the number of data pins attached to this port.
    pub fn pin_count(&self) -> u8 {
        self.properties.port_width
    }

    /// Resets the port, optionally purging the RX and/or TX buffer.
    ///
    /// Note that this will not reset all internal state.
    pub fn reset(&mut self, flags: ResetFlags) -> Result<()> {
        self.write_control(ControlReq::Reset, flags.bits(), &[])
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

impl<M: AnyBitMode> fmt::Debug for Port<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mode = type_name::<M>();
        let mode = mode.rsplit("::").next().unwrap();

        f.debug_struct(&format!("Port<{}>", mode))
            .field("index", &self.index())
            .field("timeout", &self.timeout)
            .finish()
    }
}
