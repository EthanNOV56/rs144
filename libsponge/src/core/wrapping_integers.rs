use rand::random;

use std::{fmt::Display, ops::Sub};

#[derive(Debug, Default, Clone)]
pub struct WrappingU32 {
    raw_val: u32,
}

impl Sub for WrappingU32 {
    type Output = i32;

    #[inline(always)]
    fn sub(self, other: Self) -> Self::Output {
        (self.raw_val - other.raw_val) as _
    }
}

impl PartialEq for WrappingU32 {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.raw_val == other.raw_val
    }
}

impl From<u32> for WrappingU32 {
    #[inline(always)]
    fn from(raw_val: u32) -> Self {
        Self { raw_val }
    }
}

impl Into<u32> for WrappingU32 {
    #[inline(always)]
    fn into(self) -> u32 {
        self.raw_val
    }
}

impl Display for WrappingU32 {
    #[inline(always)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw_val)
    }
}

impl Eq for WrappingU32 {}

impl WrappingU32 {
    #[inline(always)]
    pub fn new(raw_val: u32) -> Self {
        Self { raw_val }
    }

    #[inline(always)]
    pub fn random() -> Self {
        Self { raw_val: random() }
    }

    #[inline(always)]
    pub fn raw_val(&self) -> u32 {
        self.raw_val
    }

    #[inline(always)]
    pub fn wrap(n: u64, isn: &Self) -> Self {
        let raw_val = n as u32 + isn.raw_val;
        Self { raw_val }
    }

    pub fn unwrap(n: &Self, isn: &Self, check_point: u64) -> u64 {
        let offset = n.raw_val.wrapping_sub(isn.raw_val) as u64;
        let base = check_point & !0xFFFF_FFFF;

        let candidate_current = base + offset;
        let candidate_next = candidate_current + (1 << 32);
        let candidate_prev = candidate_current.wrapping_sub(1 << 32);

        let dist_current = Self::signed_distance(candidate_current, check_point);
        let dist_next = Self::signed_distance(candidate_next, check_point);
        let dist_prev = Self::signed_distance(candidate_prev, check_point);

        if dist_next.abs() < dist_current.abs() && dist_next.abs() < dist_prev.abs() {
            candidate_next
        } else if dist_prev.abs() < dist_current.abs() && dist_prev.abs() < dist_next.abs() {
            candidate_prev
        } else {
            candidate_current
        }
    }

    fn signed_distance(a: u64, b: u64) -> i64 {
        a as i64 - b as i64
    }
}
