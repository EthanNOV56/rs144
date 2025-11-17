use crate::WrappingU32;

#[derive(Debug, Clone)]
pub struct TCPConfig {
    pub capacity: usize,
    pub max_payload_size: usize,
    pub timeout_default: u16,
    pub max_retx_attempts: u32,
    pub rt_timeout: u16,
    pub recv_capacity: usize,
    pub send_capacity: usize,
    pub fixed_isn: Option<WrappingU32>,
}

impl TCPConfig {
    pub const DEFAULT_CAPACITY: usize = 64000;
    pub const MAX_PAYLOAD_SIZE: usize = 1452;
    pub const TIMEOUT_DFLT: u16 = 1000;
    pub const MAX_RETX_ATTEMPTS: u32 = 8;
}

impl Default for TCPConfig {
    fn default() -> Self {
        TCPConfig {
            capacity: Self::DEFAULT_CAPACITY,
            max_payload_size: Self::MAX_PAYLOAD_SIZE,
            timeout_default: Self::TIMEOUT_DFLT,
            max_retx_attempts: Self::MAX_RETX_ATTEMPTS,
            rt_timeout: Self::TIMEOUT_DFLT,
            recv_capacity: Self::DEFAULT_CAPACITY,
            send_capacity: Self::DEFAULT_CAPACITY,
            fixed_isn: None,
        }
    }
}

// //! Config for classes derived from FdAdapter
// class FdAdapterConfig {
//   public:
//     Address source{"0", 0};       //!< Source address and port
//     Address destination{"0", 0};  //!< Destination address and port

//     uint16_t loss_rate_dn = 0;  //!< Downlink loss rate (for LossyFdAdapter)
//     uint16_t loss_rate_up = 0;  //!< Uplink loss rate (for LossyFdAdapter)
// };
