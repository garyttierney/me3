#![feature(fn_traits)]
#![feature(fn_ptr_trait)]
#![feature(tuple_trait)]
#![feature(unboxed_closures)]
#![feature(naked_functions)]

use std::{collections::HashMap, sync::OnceLock};

use asset::AssetLoadHook;
use me3_launcher_attach_protocol::{AttachError, AttachRequest, AttachResult, Attachment};
use me3_mod_host_assets::mapping::AssetMapping;
use crate::host::{hook::thunk::ThunkPool, ModHost};

mod detour;
mod host;
mod asset;

static INSTANCE: OnceLock<usize> = OnceLock::new();
/// https://learn.microsoft.com/en-us/windows/win32/dlls/dllmain#parameters
const DLL_PROCESS_ATTACH: u32 = 1;

dll_syringe::payload_procedure! {
    fn me_attach(request: AttachRequest) -> AttachResult {
        let mut host = ModHost::new(ThunkPool::new()?);

        /// TODO: Ask game nicely to resolve this for us
        let mut asset_mapping = AssetMapping::new(HashMap::from([
            (String::from("regulation"), String::from("")),

            (String::from("data0"), String::from("")),
            (String::from("data1"), String::from("")),
            (String::from("data2"), String::from("")),
            (String::from("data3"), String::from("")),

            // Acquired from ER .exe at 1.13.0
            (String::from("testdata"), String::from("testdata/")),
            (String::from("other"), String::from("other/")),
            (String::from("mapinfotex"), String::from("other/mapinfotex/")),
            (String::from("material"), String::from("material/")),
            (String::from("shader"), String::from("shader/")),
            (String::from("shadertestdata"), String::from("testdata/Shaderbdle")),
            (String::from("debugfont"), String::from("font/")),
            (String::from("font"), String::from("font/")),
            (String::from("chrbnd"), String::from("chr/")),
            (String::from("chranibnd"), String::from("chr/")),
            (String::from("chrbehbnd"), String::from("chr/")),
            (String::from("chrtexbnd"), String::from("chr/")),
            (String::from("chrtpf"), String::from("chr/")),
            (String::from("action"), String::from("action/")),
            (String::from("actscript"), String::from("action/script/")),
            (String::from("obj"), String::from("obj/")),
            (String::from("objbnd"), String::from("obj/")),
            (String::from("map"), String::from("map/")),
            (String::from("debugmap"), String::from("map/")),
            (String::from("maphkx"), String::from("map/")),
            (String::from("maptpf"), String::from("map/")),
            (String::from("mapstudio"), String::from("map/mapstudio/")),
            (String::from("breakgeom"), String::from("map/breakgeom/")),
            (String::from("entryfilelist"), String::from("map/entryfilelist/")),
            (String::from("onav"), String::from("map/onav/")),
            (String::from("script"), String::from("script/")),
            (String::from("talkscript"), String::from("script/talk/")),
            (String::from("aiscript"), String::from("script/talk/")),
            (String::from("msg"), String::from("msg/")),
            (String::from("param"), String::from("param/")),
            (String::from("paramdef"), String::from("paramdef/")),
            (String::from("gparam"), String::from("param/drawparam/")),
            (String::from("event"), String::from("event/")),
            (String::from("menu"), String::from("menu/")),
            (String::from("menutexture"), String::from("menu/")),
            (String::from("parts"), String::from("parts/")),
            (String::from("facegen"), String::from("facegen/")),
            (String::from("cutscene"), String::from("cutscene/")),
            (String::from("movie"), String::from("movie/")),
            (String::from("wwise_mobnkinfo"), String::from("sound/")),
            (String::from("wwise_moaeibnd"), String::from("sound/")),
            (String::from("wwise_testdata"), String::from("testdata/sound/")),
            (String::from("sfx"), String::from("sfx/")),
            (String::from("sfxbnd"), String::from("sfx/")),
            (String::from("title"), String::from("")),
            (String::from("adhoc"), String::from("adhoc/")),
            (String::from("dbgai"), String::from("script_interroot/")),
            (String::from("dbgactscript"), String::from("script_interroot/action/")),
            (String::from("menuesd_dlc"), String::from("script_interroot/action/")),
            (String::from("luascriptpatch"), String::from("script_interroot/action/")),
            (String::from("asset"), String::from("asset/")),
            (String::from("expression"), String::from("expression/")),
            (String::from("regulation"), String::from("")),
        ]));

        let asset_load_results = request.packages.iter()
            .map(|p| asset_mapping.scan_directory(&p.source.0))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| AttachError("Failed to scan asset folder".to_string()))?;

        let mut asset_hook = AssetLoadHook::new(asset_mapping);
        asset_hook.attach(&mut host);

        host.attach();

        let host = ModHost::get_attached_mut();

        Ok(Attachment)
    }
}

#[no_mangle]
pub extern "stdcall" fn DllMain(instance: usize, reason: u32, _: *mut usize) -> i32 {
    if reason == DLL_PROCESS_ATTACH {
        let _ = INSTANCE.set(instance);
    }

    1
}
