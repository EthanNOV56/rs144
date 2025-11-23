use crate::{FDAdapter, FileDescriptor, TCPSegment};

use rand::random;

pub trait LossyFDAdaptor: FDAdapter {
    fn should_drop(&self, uplink: bool) -> bool {
        let cfg = self.cfg();
        let loss = if uplink {
            cfg.loss_rate_up
        } else {
            cfg.loss_rate_dn
        };
        loss != 0 && random::<u16>() < loss
    }

    // fn read(&mut self) -> Option<TCPSegment> {
    //     let ret = FileDescriptor::read(self, None).ok()?;
    //     if self.should_drop(false) {
    //         return None;
    //     }
    //     Some(ret)
    // }

    // fn write(&mut self, data: &[u8]) -> Option<()> {
    //     if self.should_drop(true) {
    //         return None;
    //     }
    //     Some(FileDescriptor::write(self, data).ok()?)
    // }
}
//     //! \brief Write to the underlying AdapterT instance, potentially dropping the datagram to be written
//     //! \param[in] seg is the packet to either write or drop
//     void write(TCPSegment &seg) {
//         if (_should_drop(true)) {
//             return;
//         }
//         return _adapter.write(seg);
//     }

//     //! \name
//     //! Passthrough functions to the underlying AdapterT instance

//     //!@{
//     void set_listening(const bool l) { _adapter.set_listening(l); }      //!< FdAdapterBase::set_listening passthrough
//     const FdAdapterConfig &config() const { return _adapter.config(); }  //!< FdAdapterBase::config passthrough
//     FdAdapterConfig &config_mut() { return _adapter.config_mut(); }      //!< FdAdapterBase::config_mut passthrough
//     void tick(const size_t ms_since_last_tick) {
//         _adapter.tick(ms_since_last_tick);
//     }  //!< FdAdapterBase::tick passthrough
//     //!@}
// };

// #endif  // SPONGE_LIBSPONGE_LOSSY_FD_ADAPTER_HH
