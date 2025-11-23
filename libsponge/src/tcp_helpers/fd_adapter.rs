use crate::{FDAdapterConfig, FileDescriptor, Milliseconds};

#[derive(Default)]
pub struct FDAdapterBase {
    cfg: FDAdapterConfig,
    listen: bool,
}

pub trait FDAdapter: FileDescriptor {
    fn base(&self) -> &FDAdapterBase;
    fn base_mut(&mut self) -> &mut FDAdapterBase;

    #[inline(always)]
    fn set_listening(&mut self, l: bool) {
        self.base_mut().listen = l;
    }

    #[inline(always)]
    fn listen(&self) -> bool {
        self.base().listen
    }

    #[inline(always)]
    fn cfg(&self) -> &FDAdapterConfig {
        &self.base().cfg
    }

    #[inline(always)]
    fn cfg_mut(&mut self) -> &mut FDAdapterConfig {
        &mut self.base_mut().cfg
    }

    #[inline(always)]
    fn tick(&mut self, elapsed: Milliseconds) {}
}
