use crate::{Address, EthernetAddress, EthernetFrame, InternetDatagram, Milliseconds};

use std::collections::{HashMap, VecDeque};

struct EthernetAddressEntry {
    caching_time: Milliseconds,
    mac_addr: EthernetAddress,
}

struct WaitingList {
    ms_since_last_arp_sent: Milliseconds,
    waiting_datagram: VecDeque<InternetDatagram>,
}

pub struct NetworkInterface {
    ethernet_addr: EthernetAddress,
    ip_addr: Address,
    frames_out: VecDeque<EthernetFrame>,
    cache: HashMap<u32, EthernetAddressEntry>,
    queue_map: HashMap<u32, WaitingList>,
}

impl NetworkInterface {
    const MAX_RETX_WAITING_TIME: usize = 5000;
    const MAX_CACHE_TIME: usize = 30000;
}
