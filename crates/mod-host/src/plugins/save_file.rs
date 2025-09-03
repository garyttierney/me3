use std::{
    alloc::{GlobalAlloc, Layout},
    fs, mem,
    path::{Path, PathBuf},
    ptr::NonNull,
};

use bevy_ecs::system::{NonSend, ResMut};
use eyre::{eyre, OptionExt};
use from_singleton::FromSingleton;
use me3_binary_analysis::{fd4_step::Fd4StepTables, pe};
use me3_launcher_attach_protocol::AttachConfig;
use me3_mod_host_assets::mapping::VfsOverrideMapping;
use me3_mod_host_types::{alloc::DlStdAllocator, vector::DlVector};
use me3_mod_protocol::Game;
use pelite::pe::{Pe, Va};
use regex::bytes::Regex;
use tracing::{error, instrument, warn, Span};

use crate::{
    app::{ExternalRes, ExternalResource, Me3App, Startup},
    executable::Executable,
    host::ModHost,
    plugins::Plugin,
};

pub struct SaveFilePlugin;

impl Plugin for SaveFilePlugin {
    fn build(&self, app: &mut Me3App) {
        let config = app.resource::<ExternalResource<AttachConfig>>();

        if config.game >= Game::EldenRing {
            app.register_system(Startup, oversized_regulation_fix_after_er);
        } else {
            app.register_system(Startup, oversized_regulation_fix_for_sdt);
        }

        app.register_system(Startup, override_savefile);
    }
}

const SL_FATAL_ERROR: &str = "could not load alternative savefile location";

#[instrument(skip_all)]
pub fn override_savefile(
    attach_config: ExternalRes<AttachConfig>,
    mut mapping: ResMut<ExternalResource<VfsOverrideMapping>>,
) -> bevy_ecs::error::Result {
    if let Some(override_name) = &attach_config.savefile {
        let savefile_dir = attach_config
            .game
            .savefile_dir()
            .ok_or_eyre("unable to locate savefile directory")?;

        let span = Span::current();
        let override_name = override_name.clone();

        mapping.add_savefile_override(savefile_dir, move |current_path| {
            let _span_guard = span.enter();

            // Panic on failure instead of loading the user's primary savefile instead
            // of the alternative one they requested.
            override_savefile_path(current_path, &override_name)
                .inspect_err(
                    |e| error!("error" = &**e, "savefile" = ?override_name, SL_FATAL_ERROR),
                )
                .expect(SL_FATAL_ERROR)
        })?;
    }

    Ok(())
}

fn override_savefile_path(
    current_path: &Path,
    override_name: &str,
) -> Result<PathBuf, eyre::Error> {
    let override_path = current_path.with_file_name(override_name);

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

fn oversized_regulation_fix_after_er(
    exe: ExternalRes<Executable>,
    step_tables: NonSend<Fd4StepTables>,
) -> Result<(), bevy_ecs::error::BevyError> {
    let apply_fn = step_tables
        .by_name("CSRegulationStep::STEP_Idle")
        .ok_or_eyre("CSRegulationStep::STEP_Idle not found")?;

    let exe = **exe;
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

fn oversized_regulation_fix_for_sdt(
    exe: ExternalRes<Executable>,
) -> Result<(), bevy_ecs::error::BevyError> {
    let text_section =
        pe::section(**exe, ".text").map_err(|e| eyre!("PE section \"{e}\" is missing"))?;
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
