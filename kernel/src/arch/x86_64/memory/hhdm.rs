use core::{fmt::Debug, ops::Add};
use limine::response::HhdmResponse;

#[derive(Clone, Copy)]
pub struct HhdmOffset(u64);

impl Debug for HhdmOffset {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "0x{:X}", self.0)
    }
}

impl From<&'static HhdmResponse> for HhdmOffset {
    fn from(value: &'static HhdmResponse) -> Self {
        Self(value.offset())
    }
}

impl From<HhdmOffset> for u64 {
    fn from(value: HhdmOffset) -> Self {
        value.0
    }
}

impl Add<HhdmOffset> for u64 {
    type Output = u64;

    fn add(self, rhs: HhdmOffset) -> Self::Output {
        self + rhs.0
    }
}
