use crate::{FDWrapper, FileDescriptor, system_call};

use anyhow::Result;
use libc::{
    IFF_NO_PI, IFF_TAP, IFF_TUN, IFNAMSIZ, O_RDWR, TUNSETIFF, c_int, c_void, ifreq, ioctl, open,
};

use std::{ffi::CString, mem::zeroed, ptr::copy_nonoverlapping};

const CLONEDEV: &str = "/dev/net/tun";

trait DeviceType: FileDescriptor {
    const IS_TUN: bool;
    const IFF_FLAG: c_int;

    fn try_new_tuntap(dev_name: &str) -> Result<FDWrapper> {
        let fd = system_call("open", || unsafe {
            open(CLONEDEV as *const str as *const i8, O_RDWR)
        })?;

        let fdw = FDWrapper::try_from(fd)?;
        let mut tun_req: ifreq = unsafe { zeroed() };
        let c_name = CString::new(dev_name)?;
        let name_bytes = c_name.as_bytes_with_nul();
        let copy_len = IFNAMSIZ.min(name_bytes.len());

        unsafe {
            copy_nonoverlapping(
                name_bytes.as_ptr() as *const i8,
                tun_req.ifr_name.as_mut_ptr(),
                copy_len,
            );
        }

        tun_req.ifr_ifru.ifru_flags = Self::IFF_FLAG as _;

        system_call("ioctl", || unsafe {
            ioctl(fdw.fd, TUNSETIFF, &tun_req as *const ifreq as *const c_void)
        })?;

        Ok(fdw)
    }
}

pub trait Tun: DeviceType {
    const IS_TUN: bool = true;
    const IFF_FLAG: c_int = IFF_TUN | IFF_NO_PI;

    fn try_new(dev_name: &str) -> Result<FDWrapper> {
        Self::try_new_tuntap(dev_name)
    }
}

pub trait Tap: DeviceType {
    const IS_TUN: bool = false;
    const IFF_FLAG: c_int = IFF_TAP | IFF_NO_PI;

    fn try_new(dev_name: &str) -> Result<FDWrapper> {
        Self::try_new_tuntap(dev_name)
    }
}
