use crate::error::CreationError;
use tokio::fs::File;
use crate::utils::get_fd;

/// Returns a file descriptor to `/dev/net/tun`. This is async wrapper around [get_fd](crate::async::get_fd)
///
/// # Remarks
/// `/dev/net/tun` is a "file" on disk, used to get access to tun/tap devices
/// through `ioctl` upgrade.
/// 1. If `/dev/net/tun` does not exists [CreationError::FileNotFound](crate::error::CreationError) error is returned (`modprobe tun`).
/// 2. If `NET_ADMIN` capabilities are not set, [CreationError::PermissionDenied](crate::error::CreationError) error is returned
/// 3. If something else prevents to open the `/dev/net/tun` [CreationError::UnableToOpenFile](crate::error::CreationError), containing the exact error.
pub(crate) fn get_fd_async() -> Result<File, CreationError> {
    let fd = get_fd()?;
    Ok(fd.into())
}