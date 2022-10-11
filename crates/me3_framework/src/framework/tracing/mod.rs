use std::ffi::CString;
use std::mem;
use std::pin::Pin;

use detour::RawDetour;
use dynasmrt::DynasmApi;
use dynasmrt::DynasmLabelApi;
use dynasmrt::ExecutableBuffer;
use dynasmrt::{dynasm, x64::Assembler};
use once_cell::sync::OnceCell;
use tracy_client::sys::___tracy_alloc_srcloc;
use windows::Win32::System::Threading::TlsAlloc;

use super::FrameworkError;
use super::FrameworkGlobal;

pub mod runtime;

pub struct Profiler {
    tls_index: u32,
    tracy: tracy_client::Client,
}

type WindowsFn = unsafe extern "C" fn();

#[repr(C)]
#[derive(Debug)]
pub struct ZoneData {
    name: u64,
    addr: Option<unsafe extern "C" fn()>,
    profiler_entry_thunk: unsafe extern "C" fn(),
    profiler_exit_thunk: unsafe extern "C" fn(),
    prologue: Option<unsafe extern "C" fn()>,
    tls_index: u32,
}

#[derive(Debug)]
pub struct ProfiledFunction {
    code_buffer: ExecutableBuffer,
    zone: Pin<Box<ZoneData>>,
    detour: RawDetour,
    name_cstr: CString,
}

#[cfg(test)]
mod test {
    use faithe::offset_of;

    use super::*;
    #[test]
    fn zone_data_layout() {
        assert_eq!(0x0, offset_of!(ZoneData, name));
        assert_eq!(0x08, offset_of!(ZoneData, addr));
        assert_eq!(0x10, offset_of!(ZoneData, profiler_entry_thunk));
        assert_eq!(0x18, offset_of!(ZoneData, profiler_exit_thunk));
        assert_eq!(0x20, offset_of!(ZoneData, prologue));
        assert_eq!(0x28, offset_of!(ZoneData, tls_index));
        assert_eq!(0x30, mem::size_of::<ZoneData>());
    }
}

impl FrameworkGlobal for Profiler {
    fn cell() -> &'static OnceCell<Self> {
        static INSTANCE: OnceCell<Profiler> = OnceCell::new();
        &INSTANCE
    }

    fn create() -> Result<Self, super::FrameworkError> {
        let tracy = tracy_client::Client::start();

        Ok(Profiler {
            tls_index: unsafe { TlsAlloc() },
            tracy,
        })
    }
}

impl Profiler {
    pub fn tracy(&self) -> &tracy_client::Client {
        &self.tracy
    }

    pub fn install_at(
        &self,
        addr: *const (),
        name: &'static str,
    ) -> Result<ProfiledFunction, FrameworkError> {
        let name_cstr = CString::new(name).expect("given name is not a valid ASCII string"); // TODO: should be a FrameworkError

        let source_loc = unsafe {
            ___tracy_alloc_srcloc(
                1,
                name_cstr.as_ptr(),
                name_cstr.to_bytes().len(),
                std::ptr::null(),
                0,
            )
        };

        let mut zone = Box::pin(ZoneData {
            name: source_loc,
            addr: None,
            profiler_entry_thunk: runtime::profiler_entry,
            profiler_exit_thunk: runtime::profiler_exit,
            prologue: None,
            tls_index: self.tls_index,
        });

        let mut ops = Assembler::new().expect("unable to create assembler");
        let data_ptr = &*zone as *const ZoneData;

        let prelude_offset = ops.offset();
        dynasm!(ops
            ; -> prelude:
            ; push rbx
            ; mov rbx, QWORD data_ptr as _
            ; jmp QWORD [rbx + 16]
            ; int3
        );

        let prologue_offset = ops.offset();
        dynasm!(ops
            ; -> prologue:
            ; mov rbx, QWORD data_ptr as _
            ; jmp QWORD [rbx + 24]
            ; int3
        );

        let code_buffer = ops
            .finalize()
            .expect("unable to create profiler prelude/prologue buffer");

        let prelude: WindowsFn = unsafe { mem::transmute(code_buffer.ptr(prelude_offset)) };
        let prologue: WindowsFn = unsafe { mem::transmute(code_buffer.ptr(prologue_offset)) };

        let detour = unsafe { RawDetour::new(addr as *const _, prelude as *const _)? };

        unsafe { detour.enable()? };

        zone.addr = Some(unsafe { mem::transmute::<*const (), WindowsFn>(detour.trampoline()) });
        zone.prologue = Some(prologue);

        Ok(ProfiledFunction {
            code_buffer,
            detour,
            name_cstr,
            zone,
        })
    }
}
