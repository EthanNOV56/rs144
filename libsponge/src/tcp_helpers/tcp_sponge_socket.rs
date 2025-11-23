use std::{sync::atomic::AtomicBool, thread};

use anyhow::Result;

use crate::{
    EventLoop, EventRule, LSSocket, TCPConfig, TCPConnection, TCPOverIPv4OverEthernetAdapter,
    TCPOverIPv4OverTunFdAdapter,
};

pub struct TCPSpongeSocket<F> {
    thread_data: LSSocket,
    dgram_adapter: A,
    tcp: Option<TCPConnection>,
    event_loop: EventLoop,
    // tcp_thread: thread,
    abort: AtomicBool,
    inbound_shutdown: bool,
    outbound_shutdown: bool,
    fully_acked: bool,
}

impl<A: Default + Clone> TCPSpongeSocket<A> {
    fn read_to_tcp(&mut self) -> Result<()> {
        let mut rule = EventRule::new(self.dgram_adapter.into(), direction, handler);
        // rule 1: read from filtered packet stream and dump into TCPConnection
        //     _eventloop.add_rule(_datagram_adapter,
        //                         Direction::In,
        //                         [&] {
        //                             auto seg = _datagram_adapter.read();
        //                             if (seg) {
        //                                 _tcp->segment_received(move(seg.value()));
        //                             }

        //                             // debugging output:
        //                             if (_thread_data.eof() and _tcp.value().bytes_in_flight() == 0 and not _fully_acked) {
        //                                 cerr << "DEBUG: Outbound stream to "
        //                                      << _datagram_adapter.config().destination.to_string()
        //                                      << " has been fully acknowledged.\n";
        //                                 _fully_acked = true;
        //                             }
        //                         },
        //                         [&] { return _tcp->active(); });

        self.event_loop.add_rule(rule);
    }

    fn init_TCP(&mut self, cfg: &TCPConfig) -> Result<()> {
        self.tcp = Some(TCPConnection::with_config(cfg)); // emplace?
        self.event_loop.add_rule(rule);

        //     // rule 2: read from pipe into outbound buffer
        //     _eventloop.add_rule(
        //         _thread_data,
        //         Direction::In,
        //         [&] {
        //             const auto data = _thread_data.read(_tcp->remaining_outbound_capacity());
        //             const auto len = data.size();
        //             const auto amount_written = _tcp->write(move(data));
        //             if (amount_written != len) {
        //                 throw runtime_error("TCPConnection::write() accepted less than advertised length");
        //             }

        //             if (_thread_data.eof()) {
        //                 _tcp->end_input_stream();
        //                 _outbound_shutdown = true;

        //                 // debugging output:
        //                 cerr << "DEBUG: Outbound stream to " << _datagram_adapter.config().destination.to_string()
        //                      << " finished (" << _tcp.value().bytes_in_flight() << " byte"
        //                      << (_tcp.value().bytes_in_flight() == 1 ? "" : "s") << " still in flight).\n";
        //             }
        //         },
        //         [&] { return (_tcp->active()) and (not _outbound_shutdown) and (_tcp->remaining_outbound_capacity() > 0); },
        //         [&] {
        //             _tcp->end_input_stream();
        //             _outbound_shutdown = true;
        //         });

        //     // rule 3: read from inbound buffer into pipe
        //     _eventloop.add_rule(
        //         _thread_data,
        //         Direction::Out,
        //         [&] {
        //             ByteStream &inbound = _tcp->inbound_stream();
        //             // Write from the inbound_stream into
        //             // the pipe, handling the possibility of a partial
        //             // write (i.e., only pop what was actually written).
        //             const size_t amount_to_write = min(size_t(65536), inbound.buffer_size());
        //             const std::string buffer = inbound.peek_output(amount_to_write);
        //             const auto bytes_written = _thread_data.write(move(buffer), false);
        //             inbound.pop_output(bytes_written);

        //             if (inbound.eof() or inbound.error()) {
        //                 _thread_data.shutdown(SHUT_WR);
        //                 _inbound_shutdown = true;

        //                 // debugging output:
        //                 cerr << "DEBUG: Inbound stream from " << _datagram_adapter.config().destination.to_string()
        //                      << " finished " << (inbound.error() ? "with an error/reset.\n" : "cleanly.\n");
        //                 if (_tcp.value().state() == TCPState::State::TIME_WAIT) {
        //                     cerr << "DEBUG: Waiting for lingering segments (e.g. retransmissions of FIN) from peer...\n";
        //                 }
        //             }
        //         },
        //         [&] {
        //             return (not _tcp->inbound_stream().buffer_empty()) or
        //                    ((_tcp->inbound_stream().eof() or _tcp->inbound_stream().error()) and not _inbound_shutdown);
        //         });

        //     // rule 4: read outbound segments from TCPConnection and send as datagrams
        //     _eventloop.add_rule(_datagram_adapter,
        //                         Direction::Out,
        //                         [&] {
        //                             while (not _tcp->segments_out().empty()) {
        //                                 _datagram_adapter.write(_tcp->segments_out().front());
        //                                 _tcp->segments_out().pop();
        //                             }
        //                         },
        //                         [&] { return not _tcp->segments_out().empty(); });
        // }
    }
}

// //! Set up the TCPConnection and the event loop
// void _initialize_TCP(const TCPConfig &config);

// //! TCP state machine
// std::optional<TCPConnection> _tcp{};

// //! eventloop that handles all the events (new inbound datagram, new outbound bytes, new inbound bytes)
// EventLoop _eventloop{};

// //! Process events while specified condition is true
// void _tcp_loop(const std::function<bool()> &condition);

// //! Main loop of TCPConnection thread
// void _tcp_main();

// //! Handle to the TCPConnection thread; owner thread calls join() in the destructor
// std::thread _tcp_thread{};

// //! Construct LocalStreamSocket fds from socket pair, initialize eventloop
// TCPSpongeSocket(std::pair<FileDescriptor, FileDescriptor> data_socket_pair, AdaptT &&datagram_interface);

// std::atomic_bool _abort{false};  //!< Flag used by the owner to force the TCPConnection thread to shut down

// bool _inbound_shutdown{false};  //!< Has TCPSpongeSocket shut down the incoming data to the owner?

// bool _outbound_shutdown{false};  //!< Has the owner shut down the outbound data to the TCP connection?

// bool _fully_acked{false};  //!< Has the outbound data been fully acknowledged by the peer?

// public:
// //! Construct from the interface that the TCPConnection thread will use to read and write datagrams
// explicit TCPSpongeSocket(AdaptT &&datagram_interface);

// //! Close socket, and wait for TCPConnection to finish
// //! \note Calling this function is only advisable if the socket has reached EOF,
// //! or else may wait foreever for remote peer to close the TCP connection.
// void wait_until_closed();

// //! Connect using the specified configurations; blocks until connect succeeds or fails
// void connect(const TCPConfig &c_tcp, const FdAdapterConfig &c_ad);

// //! Listen and accept using the specified configurations; blocks until accept succeeds or fails
// void listen_and_accept(const TCPConfig &c_tcp, const FdAdapterConfig &c_ad);

// //! When a connected socket is destructed, it will send a RST
// ~TCPSpongeSocket();

// //! \name
// //! This object cannot be safely moved or copied, since it is in use by two threads simultaneously

// //!@{
// TCPSpongeSocket(const TCPSpongeSocket &) = delete;
// TCPSpongeSocket(TCPSpongeSocket &&) = delete;
// TCPSpongeSocket &operator=(const TCPSpongeSocket &) = delete;
// TCPSpongeSocket &operator=(TCPSpongeSocket &&) = delete;
// //!@}

// //! \name
// //! Some methods of the parent Socket wouldn't work as expected on the TCP socket, so delete them

// //!@{
// void bind(const Address &address) = delete;
// Address local_address() const = delete;
// Address peer_address() const = delete;
// void set_reuseaddr() = delete;
// //!@}
// };

pub type TCPOverIPv4SpongeSocket = TCPSpongeSocket<TCPOverIPv4OverTunFdAdapter>;

pub type RS144TCPSocket = TCPOverIPv4SpongeSocket;

pub type TCPOverIPv4OverEthernetSpongeSocket = TCPSpongeSocket<TCPOverIPv4OverEthernetAdapter>;

#[derive(Default)]
pub struct FullStackSocket {
    eof_flag: bool,
}

impl FullStackSocket {
    pub fn new() -> Self {
        FullStackSocket {
            ..Default::default()
        }
    }

    pub fn connect(&self, addr: &str) -> Result<(), String> {
        // Implementation goes here
        Ok(())
    }

    pub fn write(&self, data: &[u8]) -> Result<(), String> {
        // Implementation goes here
        Ok(())
    }

    pub fn read(&self) -> Result<u8, String> {
        // Implementation goes here
        Ok(0)
    }

    pub fn wait_until_closed(&self) -> Result<(), String> {
        // Implementation goes here
        Ok(())
    }

    pub fn eof(&self) -> bool {
        self.eof_flag
    }
}
