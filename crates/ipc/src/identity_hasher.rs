use std::hash::{BuildHasher, Hasher};

#[derive(Clone, Copy, Debug)]
pub struct IdentityHasher(u64);

#[derive(Clone, Copy, Debug)]
pub struct IdentityBuildHasher;

impl IdentityHasher {
    pub fn new() -> Self {
        Self(0)
    }
}

impl Hasher for IdentityHasher {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        self.0 = bytes
            .iter()
            .enumerate()
            .take(size_of::<u64>())
            .fold(0, |acc, (i, b)| acc | ((*b as u64) << (i * 8)));
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.0 = i;
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
}

impl BuildHasher for IdentityBuildHasher {
    type Hasher = IdentityHasher;

    fn build_hasher(&self) -> Self::Hasher {
        IdentityHasher::new()
    }
}
