use std::{
    ops::{BitAnd, BitOr, Not, Sub},
    sync::atomic::{AtomicUsize, Ordering},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LanePos {
    Sync,
    Async(u8),
}

impl LanePos {
    pub fn is_sync(&self) -> bool {
        matches!(self, LanePos::Sync)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LaneMask(usize);

#[derive(Debug)]
pub struct AtomicLaneMask(AtomicUsize);

impl LaneMask {
    pub const LANE_COUNT: usize = std::mem::size_of::<usize>() * 8;
    pub const ASYNC_LANE_COUNT: usize = Self::LANE_COUNT - 1;

    pub const NONE: LaneMask = LaneMask(0);
    pub const ALL: LaneMask = LaneMask(usize::MAX);
    pub const SINGLE_SYNC: LaneMask = LaneMask(1 << (Self::LANE_COUNT - 1));
    pub const ALL_ASYNC: LaneMask = Self(!Self::SINGLE_SYNC.0);

    #[inline(always)]
    pub const fn new_single(lane_pos: LanePos) -> Self {
        match lane_pos {
            LanePos::Sync => Self::SINGLE_SYNC,
            LanePos::Async(pos) => {
                debug_assert!((pos as usize) < Self::ASYNC_LANE_COUNT);
                Self(1 << pos)
            }
        }
    }

    pub const fn new() -> Self {
        Self::NONE
    }

    #[inline(always)]
    pub const fn contains(&self, lane_pos: LanePos) -> bool {
        self.overlaps(Self::new_single(lane_pos))
    }

    #[inline(always)]
    pub const fn contains_all(&self, lanes: LaneMask) -> bool {
        self.0 & lanes.0 == lanes.0
    }

    #[inline(always)]
    pub const fn overlaps(&self, lanes: LaneMask) -> bool {
        self.0 & lanes.0 != 0
    }

    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }

    #[inline(always)]
    pub const fn count_lanes(&self) -> u32 {
        self.0.count_ones()
    }

    pub const fn iter(&self) -> impl Iterator<Item = LanePos> {
        LaneMaskIterator(*self)
    }

    // const fn from_iter<T: IntoIterator<Item = LanePos>>(iter: T) -> Self {
    //     let mut res = LaneMask::new();
    //     while
    //     for i in iter {
    //         res = res + LaneMask::new_single(i);
    //     }
    //     return res
    // }
}

impl BitOr<LaneMask> for LaneMask {
    type Output = LaneMask;

    #[inline(always)]
    fn bitor(self, rhs: LaneMask) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOr<LanePos> for LaneMask {
    type Output = LaneMask;

    #[inline(always)]
    fn bitor(self, rhs: LanePos) -> Self::Output {
        self | Self::new_single(rhs)
    }
}

impl Sub<LaneMask> for LaneMask {
    type Output = LaneMask;

    #[inline(always)]
    fn sub(self, rhs: LaneMask) -> Self::Output {
        Self(self.0 & !rhs.0)
    }
}

impl Sub<LanePos> for LaneMask {
    type Output = LaneMask;

    #[inline(always)]
    fn sub(self, rhs: LanePos) -> Self::Output {
        self - Self::new_single(rhs)
    }
}

impl BitAnd<LaneMask> for LaneMask {
    type Output = LaneMask;

    #[inline(always)]
    fn bitand(self, rhs: LaneMask) -> Self::Output {
        Self(self.0 & !rhs.0)
    }
}

impl Not for LaneMask {
    type Output = LaneMask;

    #[inline(always)]
    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

impl IntoIterator for LaneMask {
    type Item = LanePos;

    type IntoIter = LaneMaskIterator;

    fn into_iter(self) -> Self::IntoIter {
        LaneMaskIterator(self)
    }
}

pub struct LaneMaskIterator(LaneMask);

impl Iterator for LaneMaskIterator {
    type Item = LanePos;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.0.is_empty() {
            return None;
        }
        if self.0.overlaps(LaneMask::SINGLE_SYNC) {
            self.0 = self.0 & LaneMask::ALL_ASYNC;
            return Some(LanePos::Sync);
        }
        match self.0 .0.trailing_zeros() as usize {
            LaneMask::LANE_COUNT => None,
            pos => {
                self.0 .0 &= !(1 << pos);
                Some(LanePos::Async(pos as u8))
            }
        }
    }

    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.0.count_lanes() as usize;
        (len, Some(len))
    }
}

impl ExactSizeIterator for LaneMaskIterator {}

impl From<LanePos> for LaneMask {
    #[inline(always)]
    fn from(lane_pos: LanePos) -> Self {
        LaneMask::new_single(lane_pos)
    }
}

impl FromIterator<LanePos> for LaneMask {
    #[inline(always)]
    fn from_iter<T: IntoIterator<Item = LanePos>>(iter: T) -> Self {
        let mut res = LaneMask::new();
        for i in iter {
            res = res | LaneMask::new_single(i);
        }
        return res;
    }
}

impl FromIterator<LaneMask> for LaneMask {
    #[inline(always)]
    fn from_iter<T: IntoIterator<Item = LaneMask>>(iter: T) -> Self {
        let mut res = LaneMask::new();
        for i in iter {
            res = res | i;
        }
        return res;
    }
}

impl AtomicLaneMask {
    #[inline(always)]
    pub const fn new(value: LaneMask) -> Self {
        Self(AtomicUsize::new(value.0))
    }

    #[inline(always)]
    pub fn load(&self, order: Ordering) -> LaneMask {
        LaneMask(self.0.load(order))
    }

    #[inline(always)]
    pub fn store(&self, value: LaneMask, order: Ordering) {
        self.0.store(value.0, order)
    }

    #[inline(always)]
    pub fn compare_exchange(
        &self,
        current: LaneMask,
        new: LaneMask,
        success: Ordering,
        failure: Ordering,
    ) -> LaneMask {
        self.0
            .compare_exchange(current.0, new.0, success, failure)
            .map_or_else(LaneMask, LaneMask)
    }

    #[inline(always)]
    pub fn compare_exchange_weak(
        &self,
        current: LaneMask,
        new: LaneMask,
        success: Ordering,
        failure: Ordering,
    ) -> LaneMask {
        self.0
            .compare_exchange_weak(current.0, new.0, success, failure)
            .map_or_else(LaneMask, LaneMask)
    }

    #[inline(always)]
    pub fn fetch_insert(&self, val: LaneMask, order: Ordering) -> LaneMask {
        LaneMask(self.0.fetch_or(val.0, order))
    }

    #[inline(always)]
    pub fn fetch_insert_single(&self, val: LanePos, order: Ordering) -> LaneMask {
        LaneMask(self.0.fetch_or(LaneMask::new_single(val).0, order))
    }

    #[inline(always)]
    pub fn fetch_retain(&self, val: LaneMask, order: Ordering) -> LaneMask {
        LaneMask(self.0.fetch_and(val.0, order))
    }

    // pub const fn fetch_retain_single(&self, val: LanePos, order: Ordering) -> LaneMask {
    //     LaneMask(self.0.fetch_and(LaneMask::new_single(val).0, order))
    // }

    #[inline(always)]
    pub fn fetch_remove(&self, val: LaneMask, order: Ordering) -> LaneMask {
        LaneMask(self.0.fetch_and(!val.0, order))
    }

    #[inline(always)]
    pub fn fetch_remove_single(&self, val: LanePos, order: Ordering) -> LaneMask {
        LaneMask(self.0.fetch_and(!LaneMask::new_single(val).0, order))
    }
}
