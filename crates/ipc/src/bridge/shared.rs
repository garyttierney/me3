use std::ptr::NonNull;

use crate::bridge::channel::Channel;

#[repr(C)]
pub struct SharedBridge {
    pub(crate) to_parent: Channel,
    pub(crate) to_child: Channel,
}

impl SharedBridge {
    /// # Safety
    ///
    /// Ownership of `block` is transferred to the function, and it must be wholly writable and
    /// `'static`. It must not be aliased.
    pub unsafe fn new_in<'a>(block: NonNull<[u8]>) -> Option<&'a mut Self> {
        let start = block.cast::<u8>();
        let align_offset = start.align_offset(align_of::<Self>());

        if block.len() < size_of::<Self>() + align_offset {
            return None;
        }

        // SAFETY: upheld by caller.
        unsafe {
            let mut start = start.add(align_offset).cast::<Self>();

            let ptr = start.as_ptr();
            (&raw mut (*ptr).to_parent).write(Channel::new());
            (&raw mut (*ptr).to_child).write(Channel::new());

            let bridge = start.as_mut();

            let remaining = block.len() - align_offset - size_of::<Self>();
            let len = (remaining / 2).min(u32::MAX as usize) as u32;

            bridge.to_parent.init(start.add(1).cast(), len);
            bridge
                .to_child
                .init(start.add(1).cast::<u8>().add(len as usize), len);

            Some(start.as_mut())
        }
    }
}
