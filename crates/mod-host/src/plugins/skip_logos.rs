use std::{mem, ptr};

use bevy_ecs::system::Res;
use eyre::OptionExt;
use me3_binary_analysis::pe;
use me3_launcher_attach_protocol::AttachConfig;
use me3_mod_protocol::Game;
use pelite::pe::Pe as _;
use rayon::iter::{IntoParallelRefIterator as _, ParallelIterator as _};
use regex::bytes::Regex;
use windows::{
    core::{s, w},
    Win32::{
        Foundation::COLORREF,
        Graphics::Gdi::CreateSolidBrush,
        System::LibraryLoader::{GetModuleHandleW, GetProcAddress},
        UI::WindowsAndMessaging::WNDCLASSEXW,
    },
};

use crate::{
    app::{ExternalResource, PostStartup, Startup},
    executable::Executable,
    host::ModHost,
    plugins::Plugin,
};

pub struct SkipLogosPlugin;

impl Plugin for SkipLogosPlugin {
    fn build(&self, app: &mut crate::app::Me3App) {
        let config = app.resource::<ExternalResource<AttachConfig>>();

        if config.skip_logos {
            if config.game >= Game::EldenRing {
                app.register_system(PostStartup, skip_fd4_logos);
            } else {
                app.register_system(PostStartup, skip_sprj_logos);
            }
        }

        app.register_system(Startup, fix_show_window_flash);
    }
}

/// Skip logos (Dark Souls 3 and Sekiro).
pub fn skip_sprj_logos(exe: Res<ExternalResource<Executable>>) -> bevy_ecs::error::Result {
    let [text, data] = pe::sections(**exe, [".text", ".data"])
        .map_err(|e| eyre::eyre!("PE section \"{e}\" is missing"))?;

    let text = exe.get_section_bytes(text)?;
    let data = exe.get_section_bytes(data)?;

    // Matches:
    // rex push rbp
    // push   rsi
    // push   rdi
    // lea    rbp,[rsp+??]
    // sub    rsp,??
    // mov    QWORD PTR [rbp+??],-2
    // mov    QWORD PTR [rsp+??],rbx
    // mov    rdi,rcx
    // mov    BYTE PTR [rip+??],0x1
    let step_re = Regex::new(
        r"(?s-u)\x40\x55\x56\x57\x48\x8d\x6c\x24.\x48\x81\xec.{4}\x48\xc7\x45.\xfe\xff\xff\xff\x48\x89\x9c\x24.{4}\x48\x8b\xf9\xc6\x05.{4}\x01",
    )
    .unwrap();

    // Find the function in the .text section.
    let step_ptr = step_re
        .find(text)
        .map(|m| m.as_bytes().as_ptr() as usize)
        .ok_or_eyre("pattern returned no matches")?;

    let (_, data_ptrs, _) = unsafe { data.align_to::<usize>() };

    // Find a pointer to the function in the .data section.
    let step_ptr = &raw const *data_ptrs
        .par_iter()
        .find_any(|ptr| **ptr == step_ptr)
        .ok_or_eyre("no matching step pointers")?;

    // Replace the pointer to the step function with the one after it.
    //
    // Memory layout:
    // 0x00 pointer to function TitleStep::STEP_BeginLogo  step_ptr
    // 0x08 ...                                            ↑↑↑
    // 0x10 pointer to function TitleStep::STEP_BeginTitle step_ptr.add(2)
    unsafe {
        let next_step_fn = step_ptr.add(2).read();
        ptr::write(step_ptr as *mut usize, next_step_fn);
    }

    Ok(())
}

/// Skip logos (ELDEN RING and later games).
pub fn skip_fd4_logos(exe: Res<ExternalResource<Executable>>) -> bevy_ecs::error::Result {
    let [data, rdata] = pe::sections(**exe, [".data", ".rdata"])
        .map_err(|e| eyre::eyre!("PE section \"{e}\" is missing"))?;

    let data = exe.get_section_bytes(data)?;
    let rdata = exe.get_section_bytes(rdata)?;

    // "TitleStep::STEP_BeginLogo" as a UTF-16 string.
    let step_name_re = Regex::new(
        r"(?s-u)T\x00i\x00t\x00l\x00e\x00S\x00t\x00e\x00p\x00:\x00:\x00S\x00T\x00E\x00P\x00_\x00B\x00e\x00g\x00i\x00n\x00L\x00o\x00g\x00o\x00",
    )
    .unwrap();

    // Find the string in the .rdata section.
    let step_name_ptr = step_name_re
        .find(rdata)
        .map(|m| m.as_bytes().as_ptr() as usize)
        .ok_or_eyre("pattern returned no matches")?;

    let (_, data_ptrs, _) = unsafe { data.align_to::<usize>() };

    // Find a pointer to the string in the .data section.
    let step_name_ptr = &raw const *data_ptrs
        .par_iter()
        .find_any(|ptr| **ptr == step_name_ptr)
        .ok_or_eyre("no matching step pointers")?;

    // Replace the pointer to the step function before the string pointer with the one after it.
    //
    // Memory layout:
    // 0x00 pointer to function TitleStep::STEP_BeginLogo  step_name_ptr.sub(1)
    // 0x08 pointer to string "TitleStep::STEP_BeginLogo"  ↑↑↑
    // 0x10 pointer to function TitleStep::STEP_BeginTitle step_name_ptr.add(1)
    unsafe {
        let prev_step_fn = step_name_ptr.sub(1) as *mut usize;
        let next_step_fn = step_name_ptr.add(1).read();
        prev_step_fn.write(next_step_fn);
    }

    Ok(())
}

fn fix_show_window_flash() -> bevy_ecs::error::Result {
    unsafe {
        let user32 = GetModuleHandleW(w!("user32.dll"))?;

        let register_class = GetProcAddress(user32, s!("RegisterClassExW"))
            .ok_or_eyre("RegisterClassExW not found")?;

        ModHost::get_attached()
            .hook(mem::transmute::<
                _,
                unsafe extern "C" fn(*const WNDCLASSEXW) -> u16,
            >(register_class))
            .with_closure(|class, trampoline| {
                if !class.is_null() {
                    let mut class = class.read();
                    class.hbrBackground = CreateSolidBrush(COLORREF(0));
                    trampoline(&class)
                } else {
                    trampoline(class)
                }
            })
            .install()?;

        Ok(())
    }
}
