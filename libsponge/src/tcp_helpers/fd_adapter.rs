use crate::{FDAdapterConfig, Milliseconds};

use std::marker::PhantomData;

#[derive(Default)]
pub struct FDAdapterBase<T, L> {
    cfg: FDAdapterConfig,
    listen: bool,

    _type: PhantomData<T>,
    _lossy: PhantomData<L>,
}

impl<T, L> FDAdapterBase<T, L> {
    pub fn set_listening(&mut self, l: bool) {
        self.listen = l;
    }

    pub fn listen(&self) -> bool {
        self.listen
    }

    pub fn cfg(&self) -> &FDAdapterConfig {
        &self.cfg
    }

    fn cfg_mut(&mut self) -> &mut FDAdapterConfig {
        &mut self.cfg
    }

    fn tick(&mut self, elapsed: Milliseconds) {}
}
