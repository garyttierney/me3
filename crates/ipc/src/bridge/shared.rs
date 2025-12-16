use std::ptr::NonNull;

use crate::bridge::channel::Channel;

#[repr(C)]
pub struct SharedBridge {
    pub(crate) to_parent: Channel,
    pub(crate) to_child: Channel,
}

impl SharedBridge {
    pub unsafe fn new_in<'a>(block: NonNull<[u8]>) -> Option<&'a mut Self> {
        if block.len() < size_of::<Self>() {
            return None;
        }

        unsafe {
            let mut start = block.cast::<Self>();

            let ptr = start.as_ptr();
            (&raw mut (*ptr).to_parent).write(Channel::new());
            (&raw mut (*ptr).to_child).write(Channel::new());

            let bridge = start.as_mut();

            let len = ((block.len() - size_of::<Self>()) / 2).min(u32::MAX as usize) as u32;
            bridge.to_parent.init(start.add(1).cast(), len);
            bridge
                .to_child
                .init(start.add(1).byte_add(len as usize).cast(), len);

            Some(start.as_mut())
        }
    }
}
