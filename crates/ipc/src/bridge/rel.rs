use std::{fmt, hash::Hash, marker::PhantomData, num::NonZeroU32, ptr::NonNull};

use rkyv::{Archive, Deserialize, Serialize};

/// Non null pointer encoded as a byte offset inside an allocation to make it
/// position independent.
///
/// Benefits from niche optimization.
#[repr(C)]
#[derive(Archive, Serialize, Deserialize)]
pub struct RelPtr<T> {
    inner: NonZeroU32,
    marker: PhantomData<NonNull<T>>,
}

// Ensure niche optimization:
const _: () = assert!(size_of::<Option<RelPtr<()>>>() == size_of::<u32>());

impl<T> RelPtr<T> {
    /// Create a new [`RelPtr`] from a pair of pointers to and inside an allocation.
    ///
    /// # Safety
    ///
    /// Both pointers must belong to the same allocation. See [`NonNull::byte_offset_from`].
    ///
    /// # Panics
    ///
    /// If `ptr` is more than 4 gigabytes away from `origin` (offset cannot fit in a `u32`).
    pub const unsafe fn new(ptr: NonNull<T>, origin: NonNull<()>) -> Self {
        // SAFETY: upheld by caller.
        let offset = unsafe { ptr.cast::<()>().byte_offset_from_unsigned(origin) };

        if offset >= u32::MAX as usize {
            panic!("relative pointer offset exceeds 4 GB");
        }

        // SAFETY: `offset + 1` cannot overflow, because `offset` cannot be equal to `u32::MAX`.
        let inner = unsafe { NonZeroU32::new_unchecked(offset as u32 + 1) };

        Self {
            inner,
            marker: PhantomData,
        }
    }

    /// Recover the pointer passed to [`RelPtr::new`] given its origin.
    ///
    /// # Safety
    ///
    /// `origin` must be the same as the one passed to [`RelPtr::new`].
    pub const unsafe fn get(self, origin: NonNull<()>) -> NonNull<T> {
        // SAFETY: upheld by caller.
        unsafe { origin.byte_add(self.inner.get() as usize - 1).cast::<T>() }
    }
}

impl<T> Clone for RelPtr<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for RelPtr<T> {}

impl<T> fmt::Debug for RelPtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl<T> Hash for RelPtr<T> {
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl<T> PartialEq for RelPtr<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.inner.eq(&other.inner)
    }
}
