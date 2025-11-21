use crate::{BufferList, IPv4Header};

struct IPv4Datagram {
    header: IPv4Header,
    payload: BufferList,
}

pub type InternetDatagram = IPv4Datagram;

// class IPv4Datagram {
//   private:
//     IPv4Header _header{};
//     BufferList _payload{};

//   public:
//     //! \brief Parse the segment from a string
//     ParseResult parse(const Buffer buffer);

//     //! \brief Serialize the segment to a string
//     BufferList serialize() const;

//     //! \name Accessors
//     //!@{
//     const IPv4Header &header() const { return _header; }
//     IPv4Header &header() { return _header; }

//     const BufferList &payload() const { return _payload; }
//     BufferList &payload() { return _payload; }
//     //!@}
// };

// using InternetDatagram = IPv4Datagram;
