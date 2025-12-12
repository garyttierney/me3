use std::{collections::BTreeMap, mem, ptr::NonNull, slice, sync::OnceLock};

use eyre::{eyre, OptionExt};
use me3_binary_analysis::{pe, rtti::ClassMap};
use me3_launcher_attach_protocol::AttachConfig;
use me3_mod_host_types::alloc::DlAllocator;
use me3_mod_protocol::Game;
use pelite::pe::{Pe, PeObject};
use regex::bytes::Regex;
use tracing::{info, instrument};

use crate::{executable::Executable, host::ModHost};

mod mimalloc;

pub use mimalloc::MIMALLOC_DLALLOC;

#[repr(C)]
struct CSMemoryVtable {
    drop: unsafe extern "C" fn(NonNull<()>),
    init: unsafe extern "C" fn(NonNull<()>),
    deinit: unsafe extern "C" fn(NonNull<()>),
}

static SYSTEM_ALLOC_IS_HOOKED: OnceLock<bool> = OnceLock::new();

#[instrument(skip_all)]
pub fn hook_system_allocator(
    attach_config: &AttachConfig,
    exe: Executable,
) -> Result<(), eyre::Error> {
    fn hook_system_allocator_inner(exe: Executable) -> Result<(), eyre::Error> {
        let re = Regex::new(
            r"(?s-u)\xe8.{4}\x48\x8b\x74\x24\x30\x48\x8b\x5c\x24\x38\x48\x83\xc4\x20\x5f\xe9(.{4})",
        )
        .unwrap();

        let text_section =
            pe::section(exe, ".text").map_err(|_| eyre!(".text section is missing"))?;
        let text = exe.get_section_bytes(text_section)?;

        let Some((_, [disp32 @ &[b0, b1, b2, b3]])) = re.captures(text).map(|c| c.extract()) else {
            return Err(eyre!("system allocator pattern returned no matches"));
        };

        let get_system_allocator = unsafe {
            let ptr = disp32
                .as_ptr_range()
                .end
                .byte_offset(i32::from_le_bytes([b0, b1, b2, b3]) as _)
                as *const ();

            mem::transmute::<_, unsafe extern "C" fn() -> NonNull<DlAllocator>>(ptr)
        };

        ModHost::get_attached()
            .hook(get_system_allocator)
            .with({
                extern "C" fn get_allocator_override() -> NonNull<DlAllocator> {
                    NonNull::from_ref(&MIMALLOC_DLALLOC)
                }
                get_allocator_override
            })
            .install()?;

        Ok(())
    }

    if !matches!(
        attach_config.game,
        Game::DarkSouls3 | Game::Sekiro | Game::EldenRing
    ) {
        info!("game" = %attach_config.game, "skipping unsupported game");
        return Ok(());
    }

    let mut result = None;

    SYSTEM_ALLOC_IS_HOOKED.get_or_init(|| {
        info!("game" = %attach_config.game, "hooking system allocator");

        result = Some(hook_system_allocator_inner(exe));
        result.as_ref().unwrap().is_ok()
    });

    result.expect("hook_system_allocator called more than once")
}

#[instrument(skip_all)]
pub fn hook_heap_allocators(
    attach_config: &AttachConfig,
    exe: Executable,
    class_map: &ClassMap,
) -> Result<(), eyre::Error> {
    if !matches!(
        attach_config.game,
        Game::DarkSouls3 | Game::Sekiro | Game::EldenRing
    ) {
        info!("game" = %attach_config.game, "skipping unsupported game");
        return Ok(());
    }

    info!("game" = %attach_config.game, "hooking heap allocators");

    if SYSTEM_ALLOC_IS_HOOKED.get().is_none_or(|b| !b) {
        return Err(eyre!("system allocator was not hooked"));
    }

    let vtable = if let Some(vtable) = class_map
        .get("CS::CSMemoryImp")
        .or_else(|| class_map.get("NS_SPRJ::CSMemoryImp"))
        .and_then(|vtable| vtable.first())
    {
        unsafe { vtable.as_ref::<CSMemoryVtable>() }
    } else {
        find_cs_memory_vtable(exe)?
    };

    match attach_config.game {
        Game::DarkSouls3 => patch_ds3(exe),
        Game::Sekiro => patch_sdt(exe),
        Game::EldenRing => patch_er(class_map),
        _ => unreachable!("allocator hooking is not supported"),
    }?;

    let alloc_table = allocator_table_mut(attach_config, exe)?;
    alloc_table.fill(Some(NonNull::from_ref(&MIMALLOC_DLALLOC)));

    extern "C" fn nothing(_: NonNull<()>) {}

    ModHost::get_attached()
        .hook(vtable.init)
        .with(nothing)
        .install()?;

    let _ = ModHost::get_attached()
        .hook(vtable.deinit)
        .with(nothing)
        .install();

    Ok(())
}

fn find_cs_memory_vtable(exe: Executable) -> Result<&'static CSMemoryVtable, eyre::Error> {
    let vtable_re = Regex::new(
        r"(?s-u)\xe8.{4}\x90\x48\x8d\x05(.{4})\x48\x89\x03\xc6\x83.\x02\x00\x00\x00\x48\x8b\xc3\x48\x83\xc4.\x5b\xc3"
    )
    .unwrap();

    let Some((_, [disp32 @ &[b0, b1, b2, b3]])) =
        vtable_re.captures(exe.image()).map(|c| c.extract())
    else {
        return Err(eyre!("CSMemoryImp pattern returned no matches"));
    };

    unsafe {
        let vtable_ptr = disp32
            .as_ptr_range()
            .end
            .byte_offset(i32::from_le_bytes([b0, b1, b2, b3]) as _)
            as *const CSMemoryVtable;

        Ok(&*vtable_ptr)
    }
}

fn allocator_table_mut(
    attach_config: &AttachConfig,
    exe: Executable,
) -> Result<&'static mut [Option<NonNull<DlAllocator>>], eyre::Error> {
    let first_re =
        Regex::new(r"(?s-u)\x48\x89\x05(.{4})\x4c\x8b\xc0\xba\x08\x00\x00\x00\x8d\x4a\x08")
            .unwrap();

    let Some((_, [disp32 @ &[b0, b1, b2, b3]])) =
        first_re.captures(exe.image()).map(|c| c.extract())
    else {
        return Err(eyre!("first allocator pattern returned no matches"));
    };

    let first_ptr = unsafe {
        disp32
            .as_ptr_range()
            .end
            .byte_offset(i32::from_le_bytes([b0, b1, b2, b3]) as _)
            as *mut Option<NonNull<DlAllocator>>
    };

    let re_str = match attach_config.game {
        Game::DarkSouls3 => r"(?s-u)\x48\x89\x05(.{4})\x4c\x8b\xc0\xba\x08\x00\x00\x00\x8d\x4a\x78",
        Game::Sekiro => r"(?s-u)\x48\x89\x05(.{4})\x4c\x8b\xc0\xba\x08\x00\x00\x00\x8d\x4a\x70",
        _ => {
            r"(?s-u)\x48\x89\x3d(.{4})\xc7\x44\x24\x20\xff\xff\xff\xff\x45\x33\xc9\x4c\x8b\xc7\x48\x8d\x15.{4}"
        }
    };

    let last_re = Regex::new(re_str).unwrap();

    let mut last_candidates = last_re
        .captures_iter(exe.image())
        .map(|c| c.extract())
        .map(|(_, [disp32])| unsafe {
            disp32
                .as_ptr_range()
                .end
                .byte_offset(i32::from_le_bytes(disp32.try_into().unwrap()) as _)
                as *mut Option<NonNull<DlAllocator>>
        })
        .collect::<Vec<_>>();

    last_candidates.sort();

    let last_ptr = *last_candidates
        .last()
        .ok_or_eyre("last allocator pattern returned no matches")?;

    if first_ptr > last_ptr {
        return Err(eyre!("malformed allocator range"));
    }

    unsafe {
        Ok(slice::from_raw_parts_mut(
            first_ptr,
            last_ptr.offset_from_unsigned(first_ptr) + 1,
        ))
    }
}

fn patch_ds3(exe: Executable) -> Result<(), eyre::Error> {
    let re = Regex::new(
        r"(?s-u)\x48\x8b\x1d.{4}\x48\x8b\x0d.{4}\xe8(.{4})\x48\x8b\xd0\x45\x33\xc0\x48\x8b\xcb\xe8.{4}\xe8.{4}",
    )
    .unwrap();

    let text_section = pe::section(exe, ".text").map_err(|_| eyre!(".text section is missing"))?;
    let text = exe.get_section_bytes(text_section)?;

    let Some((_, [disp32 @ &[b0, b1, b2, b3]])) = re.captures(text).map(|c| c.extract()) else {
        return Err(eyre!("debug allocator getter pattern returned no matches"));
    };

    let ptr = unsafe {
        disp32
            .as_ptr_range()
            .end
            .byte_offset(i32::from_le_bytes([b0, b1, b2, b3]) as _) as *const ()
    };

    let fn_ptr = unsafe {
        mem::transmute::<_, unsafe extern "C" fn(*const ()) -> NonNull<DlAllocator>>(ptr)
    };

    ModHost::get_attached()
        .hook(fn_ptr)
        .with({
            extern "C" fn debug_allocator(_: *const ()) -> NonNull<DlAllocator> {
                NonNull::from_ref(&MIMALLOC_DLALLOC)
            }
            debug_allocator
        })
        .install()?;

    Ok(())
}

fn patch_sdt(exe: Executable) -> Result<(), eyre::Error> {
    let re = Regex::new(
        r"(?s-u)\xe8(.{4})\x48\x89\x44\x24.\x4c\x8b\xc0\xba\x08\x00\x00\x00\xb9\x90\x00\x00\x00\xe8.{4}",
    )
    .unwrap();

    let text_section = pe::section(exe, ".text").map_err(|_| eyre!(".text section is missing"))?;
    let text = exe.get_section_bytes(text_section)?;

    let (ptr, _) = re
        .captures_iter(text)
        .map(|c| c.extract())
        .map(|(_, [disp32])| unsafe {
            disp32
                .as_ptr_range()
                .end
                .byte_offset(i32::from_le_bytes(disp32.try_into().unwrap()) as _)
                as *const ()
        })
        .fold(BTreeMap::<_, usize>::new(), |mut all, ptr| {
            *all.entry(ptr).or_default() += 1;
            all
        })
        .into_iter()
        .max_by_key(|(_, i)| *i)
        .ok_or_eyre("debug allocator getter pattern returned no matches")?;

    let fn_ptr = unsafe {
        mem::transmute::<_, unsafe extern "C" fn(*const ()) -> NonNull<DlAllocator>>(ptr)
    };

    ModHost::get_attached()
        .hook(fn_ptr)
        .with({
            extern "C" fn debug_allocator(_: *const ()) -> NonNull<DlAllocator> {
                NonNull::from_ref(&MIMALLOC_DLALLOC)
            }
            debug_allocator
        })
        .install()?;

    Ok(())
}

fn patch_er(class_map: &ClassMap) -> Result<(), eyre::Error> {
    let vtable = class_map
        .get("CS::CSGraphicsImp")
        .and_then(|vtables| vtables.first())
        .ok_or_eyre("CSGraphicsImp vtable not found")?;

    let fn_ptr = unsafe { vtable.as_ref::<unsafe extern "C" fn(NonNull<()>)>() };

    ModHost::get_attached()
        .hook(*fn_ptr)
        .with({
            extern "C" fn nothing(_: NonNull<()>) {}
            nothing
        })
        .install()?;

    Ok(())
}
