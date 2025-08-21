use std::{
    alloc::{GlobalAlloc, Layout},
    fs, mem,
    path::{Path, PathBuf},
    ptr::NonNull,
    sync::Arc,
};

use eyre::{eyre, OptionExt};
use from_singleton::FromSingleton;
use me3_binary_analysis::{fd4_step::Fd4StepTables, pe};
use me3_launcher_attach_protocol::AttachConfig;
use me3_mod_host_assets::mapping::VfsOverrideMapping;
use me3_mod_host_types::{alloc::DlStdAllocator, vector::DlVector};
use me3_mod_protocol::Game;
use pelite::pe::{Pe, Va};
use regex::bytes::Regex;
use tracing::{error, info, instrument, warn, Span};

use crate::{executable::Executable, host::ModHost};

mod game;

const SL_FATAL_ERROR: &str = "could not load alternative savefile location";

#[instrument(skip_all)]
pub fn attach_override(
    attach_config: &AttachConfig,
    mapping: &mut VfsOverrideMapping,
) -> Result<(), eyre::Error> {
    if let Some(override_path) = &attach_config.saves_path {
        let savefile_dir = game::savefile_dir(attach_config.game)
            .ok_or_eyre("unable to locate savefile directory")?;

        let span = Span::current();
        let override_path = override_path.clone();

        mapping.add_savefile_override(savefile_dir, move |current_path| {
            let _span_guard = span.enter();

            let override_path = override_savefile_path(current_path, &override_path)
                .inspect_err(
                    |e| error!("error" = &**e, "saves_path" = ?override_path, SL_FATAL_ERROR),
                )
                .expect(SL_FATAL_ERROR);

            Some(override_path)
        })?;
    }

    Ok(())
}

fn override_savefile_path(
    current_path: &Path,
    override_path: &Path,
) -> Result<PathBuf, eyre::Error> {
    let override_path = if override_path.is_relative() {
        current_path.parent().unwrap().join(override_path)
    } else {
        override_path.to_owned()
    };

    if !override_path.try_exists()? {
        if let Some(parent_dir) = override_path.parent() {
            fs::create_dir_all(parent_dir)?;
        }

        if current_path.exists() {
            fs::copy(current_path, &override_path)?;
        }
    }

    Ok(override_path)
}

#[instrument(skip_all)]
pub fn oversized_regulation_fix(
    attach_config: Arc<AttachConfig>,
    exe: Executable,
    step_tables: &Fd4StepTables,
    _mapping: Arc<VfsOverrideMapping>,
) -> Result<(), eyre::Error> {
    if attach_config.game >= Game::EldenRing {
        oversized_regulation_fix_after_er(exe, step_tables)?;
    } else {
        oversized_regulation_fix_for_sdt(exe)?;
    }

    info!("applied hooks");

    Ok(())
}

fn oversized_regulation_fix_after_er(
    exe: Executable,
    step_tables: &Fd4StepTables,
) -> Result<(), eyre::Error> {
    let apply_fn = step_tables
        .by_name("CSRegulationStep::STEP_Idle")
        .ok_or_eyre("CSRegulationStep::STEP_Idle not found")?;

    // Intercept and free the raw regulation to prevent writing it to the savefile.
    ModHost::get_attached()
        .hook(apply_fn)
        .with_closure(move |p1, trampoline| unsafe {
            trampoline(p1);

            let regulation_manager = from_singleton::address_of::<CSRegulationManager>()
                .unwrap()
                .as_mut();

            if let Some(raw_regulation) = regulation_manager.raw_regulation.take() {
                let raw_regulation_len = mem::take(&mut regulation_manager.raw_regulation_len);

                if raw_regulation_len == 0 {
                    return;
                }

                match DlStdAllocator::for_object(exe, raw_regulation.as_ptr()) {
                    Ok(alloc) => alloc.dealloc(
                        raw_regulation.as_ptr(),
                        Layout::from_size_align_unchecked(raw_regulation_len, 1),
                    ),
                    Err(e) => {
                        warn!(
                            "error" = &*eyre!(e),
                            "failed to deallocate raw regulation data"
                        );
                    }
                }
            }
        })
        .install()?;

    Ok(())
}

fn oversized_regulation_fix_for_sdt(exe: Executable) -> Result<(), eyre::Error> {
    let text_section =
        pe::section(exe, ".text").map_err(|e| eyre!("PE section \"{e}\" is missing"))?;
    let text = exe.get_section_bytes(text_section)?;

    // matches:
    // lea    rcx,[rsp+0x30]
    // call   ??
    // test   al,al
    // je     ??
    let call_re = Regex::new(r"(s?-u)\x48\x8d\x4c\x24.\xe8(.{4})\x84\xc0\x74.").unwrap();

    // matches:
    // mov    rdx,rcx
    // mov    rcx,QWORD PTR [rip+??]
    // test   rcx,rcx
    // jne    ??
    // xor    al,al
    // ret
    let fn_re = Regex::new(
        r"(?s-u)\A\x48\x8b\xd1\x48\x8b\x0d.{4}\x48\x85\xc9(?:(?:\x75.)|(?:\x0f\x85.{4}))\x32\xc0\xc3",
    )
    .unwrap();

    // Intercept and skip writing the regulation to the savefile.
    call_re
        .captures_iter(text)
        .filter_map(|c| {
            let [disp32] = c.extract().1;

            let fn_ptr = disp32
                .as_ptr_range()
                .end
                .wrapping_offset(i32::from_le_bytes(disp32.try_into().unwrap()) as isize);

            if text.as_ptr_range().contains(&fn_ptr)
                && let Ok(fn_start) = exe.read_bytes(fn_ptr as Va)
                && fn_re.is_match_at(fn_start, 0)
            {
                unsafe {
                    Some(mem::transmute::<_, unsafe extern "C" fn(usize) -> bool>(
                        fn_start.as_ptr(),
                    ))
                }
            } else {
                None
            }
        })
        .try_for_each(|f| {
            ModHost::get_attached()
                .hook(f)
                .with_closure(|_, _| true)
                .install()?;

            eyre::Ok(())
        })?;

    Ok(())
}

#[repr(C)]
struct CSRegulationManager {
    _vtable: usize,
    _regulation_step: *mut (),
    _param_res_caps: DlVector<*mut ()>,
    raw_regulation: Option<NonNull<u8>>,
    raw_regulation_len: usize,
}

impl FromSingleton for CSRegulationManager {}
