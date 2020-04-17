use crate::{
    error::CreationError,
    unix::utils::{
        get_fd, InterfaceRequest, tun_set_interface,
    },
};
use std::{
    fs::File,
    fmt::{Display, Debug, Formatter, Result as FmtResult},
    io::{Read, Write, Result as IoResult},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Mode which device is running in
/// * `Tun` - Tunnel is layer 3 virtual interface, cannot be bridged. Works with IP Packets
/// * `Tap` - Terminal Access Point layer 2 virtual interface. Works with Ethernet Frames
pub enum DeviceMode {
    Tun,
    Tap,
}

impl Display for DeviceMode {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{}", if *self == DeviceMode::Tun { "tun" } else { "tap" })
    }
}

#[derive(Debug, Clone)]
/// Builder pattern to create new tun or tap device
pub struct DeviceBuilder<'a> {
    name: Option<&'a str>,
    mode: DeviceMode,
    packet_info: bool,
}

impl<'a> DeviceBuilder<'a> {
    /// Start creating new Device
    ///
    /// # Arguments
    /// * `mode` - Mode in which this device will run in
    pub fn new(mode: DeviceMode) -> Self {
        Self {
            mode,
            name: None,
            packet_info: false,
        }
    }

    /// Set name for this device.
    ///
    /// # Remarks
    ///
    /// If no name is name is specified, a new device with unique name will be created and
    /// assigned to the device.
    pub fn name(&'a mut self, name: &'a str) -> &'a mut Self {
        self.name = Some(name.as_ref());
        self
    }

    /// Set if data should contain the kernel packet info, which will be contained as 4 byte prefix
    /// of each packet
    pub fn packet_info(&'a mut self, packet_info: bool) -> &'a mut Self {
        self.packet_info = packet_info;
        self
    }

    /// Finish opening of a tun device
    ///
    /// # Errors
    /// `/dev/net/tun` is a "file" on disk, used to get access to tun/tap devices
    /// * 1. If `/dev/net/tun` does not exists [CreationError::FileNotFound](crate::error::CreationError) error is returned.
    /// * 2. If `NET_ADMIN` capabilities are not set, [CreationError::PermissionDenied](crate::error::CreationError) error is returned
    /// * 3. If something else prevents to open the `/dev/net/tun` [CreationError::UnableToOpenFile](crate::error::CreationError), containing the inner error.
    /// Name of the device must follow a strict rules, if any of those are not met [CreationError::InvalidName](crate::error::CreationError) is returned:
    /// * 1. Interface name *MUST* contains only ASCII characters
    /// * 2. Interface name *MUST NOT* contain `0` value (null terminator)
    /// * 2. Interface name *MUST* be shorter than `IFNAMSIZ` (shorter, because last char is null terminator)
    /// If ioctl call fail, [CreationError::IoctlError](crate::error::CreationError) with inner ErrNo is returned.
    pub fn open(&self) -> Result<Device, CreationError> {
        use libc::{IFF_TUN, IFF_TAP, IFF_NO_PI, c_short, c_int};

        // Get file descriptor to /dev/net/tun
        let file = get_fd()?;

        // Build correct flags for ifreq
        let mut ifr_flags: c_int = 0x0;
        if self.mode == DeviceMode::Tun {
            ifr_flags |= IFF_TUN;
        } else {
            ifr_flags |= IFF_TAP;
        }

        if !self.packet_info {
            ifr_flags |= IFF_NO_PI;
        }

        let mut ifreq = InterfaceRequest::tun_set_request(if let Some(name) = self.name {
            name
        } else {
            ""
        }, ifr_flags as c_short)?;

        tun_set_interface(&file, &mut ifreq)?;

        let name = ifreq.get_name().to_string()?;

        Ok(Device {
            file,
            name,
            mode: self.mode,
        })
    }
}

/// Network tun or tap device, created with [DeviceBuilder].
pub struct Device {
    file: File,
    mode: DeviceMode,
    name: String,
}

impl Device {
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Display for Device {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{}", self.name)
    }
}

impl Debug for Device {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{:?}Device({})", self.mode, self.name)
    }
}

impl Write for Device {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.file.write(buf)
    }

    fn flush(&mut self) -> IoResult<()> {
        self.file.flush()
    }
}

impl Read for Device {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        self.file.read(buf)
    }
}
