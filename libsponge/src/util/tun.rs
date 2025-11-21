use crate::{FileDescriptor, system_call};

use anyhow::Result;
use libc::{IFF_NO_PI, IFF_TAP, IFF_TUN, IFNAMSIZ, O_RDWR, TUNSETIFF, c_void, ifreq, ioctl, open};

use std::{ffi::CString, mem::zeroed, ptr::copy_nonoverlapping};

const CLONEDEV: &str = "/dev/net/tun";

struct TunTapT;
pub type TunTapFD<A, P> = FileDescriptor<A, TunTapT, P>;

impl<A: Default, P> TunTapFD<A, P> {
    pub fn try_new(dev_name: &str, is_tun: bool) -> Result<Self> {
        let fd = system_call("open", || unsafe {
            open(CLONEDEV as *const str as *const i8, O_RDWR)
        })?;
        let this = FileDescriptor::from(fd);
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

        let flags = if is_tun { IFF_TUN } else { IFF_TAP } | IFF_NO_PI;

        tun_req.ifr_ifru.ifru_flags = flags as _;

        system_call("ioctl", || unsafe {
            ioctl(
                this.fd(),
                TUNSETIFF,
                &tun_req as *const ifreq as *const c_void,
            )
        })?;

        Ok(this)
    }
}

// struct TunT;
// pub type TunFD<A, P> = FileDescriptor<A, TunT, P>;

// struct TapT;
// pub type TapTFD<A, P> = FileDescriptor<A, TapT, P>;
