use libc::*;
use crate::{
    error::*,
};
use std::{
    io::ErrorKind,
    os::unix::io::AsRawFd,
    fs::{OpenOptions, File},
};

/// Returns a file descriptor to `/dev/net/tun`.
pub(crate) fn get_fd() -> Result<File, CreationError> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/net/tun")
        .map_err(|e| {
            match e.kind() {
                ErrorKind::NotFound => CreationError::FileNotFound,
                ErrorKind::PermissionDenied => CreationError::PermissionDenied,
                kind => CreationError::UnableToOpenFile(kind.into())
            }
        })
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
/// Structure representing name of specific network device.
/// It is a C-like buffer with constant length, containing C-like
/// nul terminated string (which must be valid).
pub struct InterfaceName {
    pub name: [c_char; IFNAMSIZ],
}

impl InterfaceName {
    /// Create empty interface name. Empty names are used, if user wants device to be assigned default name.
    pub fn empty() -> Self {
        Self {
            name: [0 as c_char; IFNAMSIZ],
        }
    }

    /// Try to create InterfaceName from Rust string.
    ///
    /// # Args
    /// * `name` - A name of device
    ///
    /// # Errors
    /// `name` must be valid ASCII string shorter than `IFNAMESIZE` (as last byte must be nul terminator),
    /// also `name` cannot contain nul terminator inside itself.
    pub fn from_str<S: AsRef<str>>(name: S) -> Result<Self, StringError> {
        use StringError::*;
        let name = name.as_ref();
        if name.len() == 0 {
            return Ok(Self::empty());
        }
        // 1. check that str is ascii only and it does not contains nul terminator inside
        if let Some(pos) = name.chars().position(|x| !x.is_ascii() || (x as u8) != 0) {
            return Err(InvalidCharacter(pos));
        }
        // 2. Check if it is not too long.
        if name.len() >= IFNAMSIZ {
            return Err(StringTooLong(IFNAMSIZ));
        }
        let mut buf = [0 as c_char; IFNAMSIZ];

        for (chr, val) in buf.iter_mut().zip(name.bytes()) {
            *chr = val as c_char;
        }

        Ok(Self {
            name: buf,
        })
    }

    /// Try to represent InterfaceName as Rust String.
    ///
    /// # Errors
    ///
    /// If nul terminator is not present in name [StringError::MangledString] is returned.
    /// If name contains non-ascii character [StringError::InvalidCharacter] is returned.
    pub fn to_string(&self) -> Result<String, StringError> {
        use StringError::*;
        let end = self.name.iter().position(|x| *x == 0)
            .ok_or(MangledString)?;
        let slice: &[i8] = &self.name[..end];
        let mut ret = String::with_capacity(slice.len());
        for (pos, byte) in slice.iter().enumerate() {
            let chr = *byte as u8 as char;
            if chr.is_ascii() {
                ret.push(chr);
            } else {
                Err(InvalidCharacter(pos))?;
            }
        }
        Ok(ret)
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
/// Interface Memory Mapping, currently only used as part of IFFRU
pub struct InterfaceMap {
    pub mem_start: c_ulong,
    pub mem_end: c_ulong,
    pub base_addr: c_ushort,
    pub irq: c_uchar,
    pub dma: c_uchar,
    pub port: c_uchar,
}

#[repr(C)]
#[derive(Copy, Clone)]
/// Part of the request describing change on requested interface device
pub union InterfaceFieldReplaceUnit {
    address: sockaddr,
    destination_address: sockaddr,
    broadcast_address: sockaddr,
    netmask: sockaddr,
    hw_address: sockaddr,
    flags: c_short,
    if_index: c_int,
    metric: c_int,
    mtu: c_int,
    map: InterfaceMap,
    slave: InterfaceName,
    new_name: InterfaceName,
    data: *mut c_void,
}

impl InterfaceFieldReplaceUnit {
    fn new() -> Self {
        unsafe {
            std::mem::zeroed()
        }
    }

    /// Create IFFRU to replace existing flags with some other
    pub fn flags<T: Into<c_short>>(flags: T) -> Self {
        let mut ret = Self::new();
        ret.flags = flags.into();
        ret
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
/// Generic interface request used in ioctl calls, used for manipulation of network devices.
pub struct InterfaceRequest {
    name: InterfaceName,
    fru: InterfaceFieldReplaceUnit,
}

impl InterfaceRequest {
    /// Get name of this request, which is stored in C-like buffer, and is needed to be propely checked
    /// and casted to Rust string
    pub fn get_name(&self) -> &InterfaceName {
        &self.name
    }

    /// Create new request to upgrade a file descriptor to tun/tap device.
    ///
    /// # Arguments
    ///
    /// * `device_name` - Name of the device requested to be upgraded into. If empty, ioctl call will assign some.
    /// * `flags` - Defines mode and properties of opened device one of `IFF_TUN` and `IFF_TAP` is required.
    ///
    /// # Errors
    ///
    /// If `device_name` is invalid ASCII string or is longer than `IFNAMSIZ` error is return describing
    /// whats wrong with the name.
    pub fn tun_set_request<S: AsRef<str>, T: Into<c_short>>(device_name: S, flags: T) -> Result<Self, StringError> {
        Ok(Self {
            name: InterfaceName::from_str(device_name)?,
            fru: InterfaceFieldReplaceUnit::flags(flags),
        })
    }
}

/// Upgrade file descriptor to bind to a device described in the InterfaceRequest.
///
/// # Arguments
///
/// * `file` - An opened `/dev/net/tun` file, you wish to upgrade to specific tun/tap device.
/// * `request` - A request containing info about device you want to upgrade to. If request contains empty name, some will be assigned to it
///
/// # Returns
///
/// If anything is wrong with the upgrade, [CreationError::IoctlError](crate::error::CreationError) is returned
/// containing an Linux error-code.
///
/// # Remarks
///
/// This is "safe-ish" wrapper around ioctl(TUNSETIFF) call. Despite the name, created device mode is
/// determined by flags in request if (IFF_TUN or IFF_TAP)
/// It is required, that given file has a filed descriptor for `/dev/net/tun` and request was made with [InterfaceRequest::tun_set_request](self::InterfaceRequest::tun_set_request),
/// otherwise it is not guaranteed to work and error code is more or less orientational as it is just wraps
/// linux `errno()`.
pub fn tun_set_interface(file: &File, request: &mut InterfaceRequest) -> Result<(), CreationError> {
    let fd = file.as_raw_fd();
    let ptr = request as *const _ as u64;
    unsafe {
        ioctl::tunsetiff(fd, ptr)?;
    }
    Ok(())
}

/// IOCTL calls (which are more or less a black magic) are unsafe and hard to use, that's why
/// they are in such restrictive module, which allows calling them only from wrappers defined util.rs.
pub(self) mod ioctl {
    use nix::ioctl_write_int;
    // ioctl(fd, TUNSETIFF, ifreq) -> Used to setup the tun/tap device on
    // opened file descriptor of /dev/net/tun
    ioctl_write_int!(tunsetiff, b'T', 202);
    // ioctl(fd, TUNSETPERSIST, {1, 2}) -> Set opened tun/tap device to persistent mode
    // the device won't be dropped after closing application
    ioctl_write_int!(tunsetpersist, b'T', 203);
    // ioctl(fd, TUNSETOWNER, uid) -> Set owner of opened tun/tap device to user with given UID.
    ioctl_write_int!(tunsetowner, b'T', 204);
    // ioctl(fd, TUNSETGROUP, gid) -> Set owning group of opened tun/tap device to group with given GID.
    ioctl_write_int!(tunsetgroup, b'T', 206);
}
