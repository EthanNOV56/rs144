use crate::{FDAdaptor, ToI};

trait ToIoT: ToI {}
trait ToIoE: ToI {}

impl ToI for TCPOverIPv4OverTUN {}
impl ToI for TCPOverIPv4OverEthernet {}

struct TCPOverIPv4OverTUN;
impl ToIoT for TCPOverIPv4OverTUN {}

struct TCPOverIPv4OverEthernet;
impl ToIoE for TCPOverIPv4OverEthernet {}

pub type TCPOverIPv4OverTunFdAdapter = FDAdaptor<TCPOverIPv4OverTUN>;

pub type TCPOverIPv4OverEthernetAdapter = FDAdaptor<TCPOverIPv4OverEthernet>;
