use crate::FileDescriptor;

struct TunT;
pub type TunFD<A, P> = FileDescriptor<A, TunT, P>;

struct TapT;
pub type TapTFD<A, P> = FileDescriptor<A, TapT, P>;

struct TunTapT;
pub type TunTapFD<A, P> = FileDescriptor<A, TunTapT, P>;
