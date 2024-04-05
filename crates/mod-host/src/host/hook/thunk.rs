use std::{
    any::Any,
    arch::asm,
    cmp::max,
    error::Error,
    marker::Tuple,
    mem::{offset_of, size_of, transmute},
    ptr::NonNull,
    slice,
    sync::atomic::{AtomicUsize, Ordering},
};

use retour::Function;
use windows::Win32::System::{
    Diagnostics::Debug::FlushInstructionCache,
    Memory::{
        VirtualAlloc, VirtualProtect, MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READ, PAGE_READWRITE,
    },
    SystemInformation::{GetSystemInfo, SYSTEM_INFO},
    Threading::GetCurrentProcess,
};

#[naked]
pub unsafe extern "C" fn thunk_info() -> *const ThunkInfo {
    asm!("mov rax, gs:[0]", "ret", options(noreturn))
}

pub unsafe fn thunk_data<T>() -> Option<NonNull<T>> {
    thunk_info()
        .as_ref()
        .and_then(|info| Some(info.data?.cast()))
}

// #[naked]
// pub unsafe extern "C" fn thunk_context<T>() -> *const T {
//     let data_ptr = get_thunk_data();
//     let data = data_ptr.as_ref().expect("ThunkData was null?");
// }

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

#[derive(Debug, Default)]
pub struct ThunkCounter {
    index: AtomicUsize,
    capacity: usize,
}

impl ThunkPool {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        use iced_x86::code_asm::*;

        let mut sysinfo = SYSTEM_INFO::default();
        unsafe { GetSystemInfo(&mut sysinfo) };
        let page_size = sysinfo.dwPageSize as usize;

        let mut a = CodeAssembler::new(64)?;
        let mut start = a.create_label();
        let info_ptr = ptr(start) + page_size - 1;
        let trampoline_ptr = info_ptr + offset_of!(ThunkInfo, trampoline);

        a.set_label(&mut start)?;
        a.lea(rax, info_ptr)?;
        a.mov(qword_ptr(0).gs(), rax)?;
        a.mov(rax, qword_ptr(trampoline_ptr))?;
        a.jmp(rax)?;

        let thunk = a.assemble(0)?;
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
            .ok_or_else(|| "VirtualAlloc failed".to_string())?;

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

    pub fn get<F: Function>(&self, closure: impl Fn<F::Arguments, Output = F::Output>) -> F
    where
        F::Arguments: Tuple,
    {
        let (thunk, _) = self.get_with_data(closure, 0usize);
        thunk
    }

    pub fn get_with_data<F: Function, T: Sized>(
        &self,
        closure: impl Fn<F::Arguments, Output = F::Output>,
        extra_data: T,
    ) -> (F, NonNull<T>)
    where
        F::Arguments: Tuple,
    {
        let index = self.counter.advance().expect("capacity reached");
        let thunk_offset = index * self.stride;

        let hook = dispatcher::<F> as *const ();
        let boxed_closure = Box::new(closure) as Box<dyn Fn<F::Arguments, Output = F::Output>>;
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

        (thunk, data)
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
                .compare_exchange(index, index + 1, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                return Some(index);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test1() {
        let allocator = ThunkPool::new().expect("failed to create allocator");
        let (func, _) =
            allocator.get_with_data::<extern "system" fn(i32) -> i32, ()>(|value| value, ());
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
