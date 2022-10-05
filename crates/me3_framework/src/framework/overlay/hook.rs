use std::{intrinsics::transmute, sync::Once};

use detour::GenericDetour;
use egui::Context;
use egui_d3d11::DirectX11App;
use faithe::internal::find_pattern;
use faithe::pattern::Pattern;
use lazy_static::lazy_static;
use windows::{
    core::HRESULT,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Dxgi::{Common::DXGI_FORMAT, IDXGISwapChain},
        UI::WindowsAndMessaging::{CallWindowProcW, SetWindowLongPtrA, GWLP_WNDPROC, WNDPROC},
    },
};

use super::Overlay;
use crate::framework::{FrameworkError, FrameworkGlobal};

static mut APP: DirectX11App<i32> = DirectX11App::new();
static mut OLD_WND_PROC: Option<WNDPROC> = None;

pub type FnPresent = unsafe extern "stdcall" fn(IDXGISwapChain, u32, u32) -> HRESULT;

static mut O_PRESENT: Option<GenericDetour<FnPresent>> = None;

pub type FnResizeBuffers =
    unsafe extern "stdcall" fn(IDXGISwapChain, u32, u32, u32, DXGI_FORMAT, u32) -> HRESULT;

static mut O_RESIZE_BUFFERS: Option<GenericDetour<FnResizeBuffers>> = None;

unsafe extern "stdcall" fn hk_present(
    swap_chain: IDXGISwapChain,
    sync_interval: u32,
    flags: u32,
) -> HRESULT {
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        APP.init_default(&swap_chain, present_overlay);

        let desc = swap_chain.GetDesc().unwrap();
        if desc.OutputWindow.0 == -1 {
            panic!("Invalid window handle");
        }

        OLD_WND_PROC = Some(transmute(SetWindowLongPtrA(
            desc.OutputWindow,
            GWLP_WNDPROC,
            hk_wnd_proc as usize as _,
        )));
    });

    APP.present(&swap_chain);

    O_PRESENT
        .as_ref()
        .unwrap()
        .call(swap_chain, sync_interval, flags)
}

unsafe extern "stdcall" fn hk_resize_buffers(
    swap_chain: IDXGISwapChain,
    buffer_count: u32,
    width: u32,
    height: u32,
    new_format: DXGI_FORMAT,
    swap_chain_flags: u32,
) -> HRESULT {
    APP.resize_buffers(&swap_chain, || {
        O_RESIZE_BUFFERS.as_ref().unwrap().call(
            swap_chain.clone(),
            buffer_count,
            width,
            height,
            new_format,
            swap_chain_flags,
        )
    })
}

unsafe extern "stdcall" fn hk_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    APP.wnd_proc(msg, wparam, lparam);

    CallWindowProcW(OLD_WND_PROC.unwrap(), hwnd, msg, wparam, lparam)
}

lazy_static! {
    static ref STEAM_GAMEOVERLAY_PRESENT_PATTERN: Pattern =
        Pattern::from_ida_style("48 89 6C 24 18 48 89 74 24 20 41 56 48 83 EC 20 41");
    static ref STEAM_GAMEOVERLAY_RESIZE_BUFFERS_PATTERN: Pattern = Pattern::from_ida_style(
        "48 89 5C 24 08 48 89 6C 24 10 48 89 74 24 18 57 41 56 41 57 48 83 EC 30 44",
    );
    static ref DXGI_PRESENT_PATTERN: Pattern =
        Pattern::from_ida_style("48 89 5C 24 10 48 89 74 24 20 55 57 41 56");
    static ref DXGI_RESIZE_BUFFERS_PATTERN: Pattern =
        Pattern::from_ida_style("48 8B C4 55 41 54 41 55 41 56 41 57 48 8D 68 B1 48 81 EC C0");
}

fn present_overlay(ctx: &Context, _i: &mut i32) {
    // UNSAFE: we're inside the overlay callback, the overlay must exist here.
    let overlay = unsafe { Overlay::get_unchecked() };

    overlay.render(ctx);
}

pub(super) fn install_overlay_hooks() -> Result<(), FrameworkError> {
    let present = find_pattern(
        "gameoverlayrenderer64.dll",
        STEAM_GAMEOVERLAY_PRESENT_PATTERN.clone(),
    )
    .or_else(|_| find_pattern("dxgi.dll", DXGI_PRESENT_PATTERN.clone()))?
    .ok_or(FrameworkError::NoMatchesFound {
        identifier: "present",
        pattern: STEAM_GAMEOVERLAY_PRESENT_PATTERN.clone(),
    })?
    .as_ptr() as usize;

    let swap_buffers = find_pattern(
        "gameoverlayrenderer64.dll",
        STEAM_GAMEOVERLAY_RESIZE_BUFFERS_PATTERN.clone(),
    )
    .or_else(|_| find_pattern("dxgi.dll", DXGI_RESIZE_BUFFERS_PATTERN.clone()))?
    .ok_or(FrameworkError::NoMatchesFound {
        identifier: "resize buffers",
        pattern: STEAM_GAMEOVERLAY_RESIZE_BUFFERS_PATTERN.clone(),
    })?
    .as_ptr() as usize;

    unsafe {
        let present_hook = GenericDetour::<FnPresent>::new(
            transmute::<_, FnPresent>(present) as _,
            hk_present as FnPresent,
        )?;

        present_hook.enable()?;

        let swap_buffers_hook = GenericDetour::<FnResizeBuffers>::new(
            transmute::<_, FnResizeBuffers>(swap_buffers),
            hk_resize_buffers as FnResizeBuffers,
        )?;

        swap_buffers_hook.enable()?;

        O_RESIZE_BUFFERS = Some(swap_buffers_hook);
        O_PRESENT = Some(present_hook);
    }

    Ok(())
}
