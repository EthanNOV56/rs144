pub trait TCPState {}
pub struct Closed;
pub struct Listen;
pub struct SynSent;
pub struct SynReceived;
pub struct Established;
pub struct FinWait1;
pub struct FinWait2;
pub struct Closing;
pub struct CloseWait;
pub struct LastAck;
pub struct TimeWait;

impl TCPState for Closed {}
impl TCPState for Listen {}
impl TCPState for SynSent {}
impl TCPState for SynReceived {}
impl TCPState for Established {}
impl TCPState for FinWait1 {}
impl TCPState for FinWait2 {}
impl TCPState for Closing {}
impl TCPState for CloseWait {}
impl TCPState for LastAck {}
impl TCPState for TimeWait {}

pub trait SenderState {}
pub struct SenderIdle;
pub struct SenderSynSent;
pub struct SenderEstablished;
pub struct SenderFinSent;
pub struct SenderFinAcknowledged;

impl SenderState for SenderIdle {}
impl SenderState for SenderSynSent {}
impl SenderState for SenderEstablished {}
impl SenderState for SenderFinSent {}
impl SenderState for SenderFinAcknowledged {}

pub trait ReceiverState {}
pub struct ReceiverIdle;
pub struct ReceiverSynReceived;
pub struct ReceiverEstablished;
pub struct ReceiverCloseWait;
pub struct ReceiverLastAck;

impl ReceiverState for ReceiverIdle {}
impl ReceiverState for ReceiverSynReceived {}
impl ReceiverState for ReceiverEstablished {}
impl ReceiverState for ReceiverCloseWait {}
impl ReceiverState for ReceiverLastAck {}
