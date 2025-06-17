use std::{
    any::Any,
    arch::asm,
    cmp::max,
    marker::Tuple,
    mem::{offset_of, size_of, transmute},
    ptr::NonNull,
    slice,
    sync::atomic::{compiler_fence, AtomicUsize, Ordering},
};

use eyre::OptionExt;
use retour::Function;
use windows::Win32::System::{
    Diagnostics::Debug::FlushInstructionCache,
    Memory::{
        VirtualAlloc, VirtualProtect, MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READ, PAGE_READWRITE,
    },
    SystemInformation::{GetSystemInfo, SYSTEM_INFO},
    Threading::GetCurrentProcess,
};

pub unsafe fn thunk_info() -> *const ThunkInfo {
    unsafe {
        let thunk_info: *const ThunkInfo;
        asm!("mov {}, gs:[0x28]", out(reg) thunk_info);
        compiler_fence(Ordering::SeqCst);
        thunk_info
    }
}

#[allow(unused)]
pub unsafe fn thunk_data<T>() -> Option<NonNull<T>> {
    thunk_info()
        .as_ref()
        .and_then(|info| Some(info.data?.cast()))
}

pub unsafe extern "rust-call" fn dispatcher<F: Function + 'static>(args: F::Arguments) -> F::Output
where
    F::Arguments: Tuple,
{
    let data_ptr = thunk_info();
    let data = data_ptr.as_ref().expect("ThunkData was null?");
    let erased_closure = &*data.closure;
    let closure: &dyn Fn<F::Arguments, Output = F::Output> = transmute(erased_closure);

    std::ops::Fn::call(closure, args)
}

unsafe impl Sync for ThunkPool {}
unsafe impl Send for ThunkPool {}

#[derive(Debug)]
pub struct ThunkPool {
    /// A counter tracking the number of thunks handed out by this pool.
    counter: ThunkCounter,

    /// Pointer to the code region for this allocator.
    code_ptr: *mut u8,

    /// Pointer to the data region for this allocator. Copies of `[ThunkData]` are stored here.
    data_ptr: *mut u8,

    /// The distance in bytes between elements stored in each region.
    stride: usize,
}

pub struct ThunkInfo {
    /// A function pointer that this thunk forwards to.
    trampoline: *const (),

    /// A fat pointer to a closure for this thunk.
    closure: *mut dyn Any,

    /// A pointer to any additional data that was stored alongside this thunk. May be retrieved by
    /// `[thunk_context]`
    data: Option<NonNull<()>>,
}

impl ThunkInfo {
    #[allow(unused)]
    pub fn trampoline(&self) -> *const () {
        self.trampoline
    }
}

#[derive(Debug, Default)]
pub struct ThunkCounter {
    index: AtomicUsize,
    capacity: usize,
}

impl ThunkPool {
    pub fn new() -> Result<Self, eyre::Error> {
        let mut sysinfo = SYSTEM_INFO::default();
        unsafe { GetSystemInfo(&mut sysinfo) };
        let page_size = sysinfo.dwPageSize as usize;

        let thunk = thunk_prototype(page_size, page_size + offset_of!(ThunkInfo, trampoline))?;

        let (code_ptr, data_ptr) = unsafe {
            let memory = NonNull::new(
                VirtualAlloc(
                    None,
                    page_size * 2,
                    MEM_COMMIT | MEM_RESERVE,
                    PAGE_READWRITE,
                )
                .cast::<u8>(),
            )
            .ok_or_eyre("VirtualAlloc failed")?;

            let code_ptr = memory.as_ptr();
            let data_ptr = code_ptr.byte_add(page_size);

            (code_ptr, data_ptr)
        };

        let element_size = max(thunk.len(), size_of::<ThunkInfo>());
        let stride = unsafe {
            let padding = code_ptr
                .byte_add(element_size)
                .align_offset(std::mem::size_of::<usize>());

            padding + element_size
        };

        let code = unsafe { slice::from_raw_parts_mut(code_ptr, page_size) };

        // Padding with int3.
        code.fill(0xCC);

        for thunk_copy in code.chunks_exact_mut(stride) {
            thunk_copy[..thunk.len()].copy_from_slice(&thunk);
        }

        let mut _old_protect = PAGE_READWRITE;
        unsafe {
            VirtualProtect(
                code_ptr.cast(),
                page_size,
                PAGE_EXECUTE_READ,
                &mut _old_protect,
            )?;

            FlushInstructionCache(GetCurrentProcess(), Some(code_ptr.cast()), page_size)?;
        }

        Ok(Self {
            code_ptr,
            data_ptr,
            counter: ThunkCounter::with_capacity(page_size / element_size),
            stride,
        })
    }

    pub fn get<F, T>(
        &self,
        boxed_closure: Box<dyn Fn<F::Arguments, Output = F::Output>>,
        extra_data: T,
    ) -> Option<(F, NonNull<T>)>
    where
        F::Arguments: Tuple,
        F: Function,
        T: Sized,
    {
        let index = self.counter.advance()?;
        let thunk_offset = index * self.stride;

        let hook = dispatcher::<F> as *const ();
        let closure = Box::into_raw(boxed_closure);

        let data = unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(extra_data))) };

        let thunk = unsafe {
            let info = ThunkInfo {
                trampoline: hook,
                closure: std::mem::transmute(closure),
                data: Some(data.cast()),
            };

            let thunk_info_ptr = self.data_ptr.byte_add(thunk_offset).cast::<ThunkInfo>();
            thunk_info_ptr.write(info);

            std::mem::transmute_copy(&self.code_ptr.byte_add(thunk_offset))
        };

        Some((thunk, data))
    }
}

impl ThunkCounter {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            index: AtomicUsize::default(),
            capacity,
        }
    }

    pub fn advance(&self) -> Option<usize> {
        loop {
            let index = self.index.load(Ordering::Relaxed);

            if index >= self.capacity {
                return None;
            }

            if self
                .index
                .compare_exchange_weak(index, index + 1, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                return Some(index);
            }
        }
    }
}

fn thunk_prototype(info_offset: usize, trampoline_offset: usize) -> Result<Vec<u8>, eyre::Error> {
    #[rustfmt::skip]
    const SHELLCODE: [u8; 22] = [
        // lea rax,[rip+SHELLCODE]
        0x48, 0x8d, 0x05, 0xF9, 0xFF, 0xFF, 0xFF,

        // mov gs:[0x28],rax
        0x65, 0x48, 0x89, 0x04, 0x25, 0x28, 0x00, 0x00, 0x00,

        // jmp [rip+SHELLCODE]
        0xFF, 0x25, 0xEA, 0xFF, 0xFF, 0xFF,
    ];

    const MOV_DISP: i32 = i32::from_le_bytes([0xF9, 0xFF, 0xFF, 0xFF]);
    const JMP_DISP: i32 = i32::from_le_bytes([0xEA, 0xFF, 0xFF, 0xFF]);

    let mut shellcode = SHELLCODE[..3].to_owned();

    shellcode.extend_from_slice(
        &MOV_DISP
            .checked_add(i32::try_from(info_offset)?)
            .ok_or_eyre("offset too big")?
            .to_le_bytes(),
    );

    shellcode.extend_from_slice(&SHELLCODE[7..18]);

    shellcode.extend_from_slice(
        &JMP_DISP
            .checked_add(i32::try_from(trampoline_offset)?)
            .ok_or_eyre("offset too big")?
            .to_le_bytes(),
    );

    Ok(shellcode)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test1() {
        let allocator = ThunkPool::new().expect("failed to create allocator");
        let (func, _) = allocator
            .get::<extern "system" fn(i32) -> i32, ()>(Box::new(|value| value), ())
            .expect("failed to get thunk");

        assert_eq!(1, func(1));
    }

    #[test]
    fn test_counter() {
        let counter = ThunkCounter::with_capacity(2);
        assert_eq!(Some(0), counter.advance());
        assert_eq!(Some(1), counter.advance());
        assert_eq!(None, counter.advance());
    }
}
