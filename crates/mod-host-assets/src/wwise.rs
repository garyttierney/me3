use crate::mapping::ArchiveOverrideMapping;

const PREFIXES: &[&str] = &["sd", "sd/enus", "sd/ja"];

/// Strip sd:/ and sd_dlc02:/ prefixes from the input string.
pub fn strip_prefix(input: &str) -> &str {
    let mut start = 0;
    loop {
        let mut found = false;
        for prefix in &["sd:/", "sd_dlc02:/"] {
            if input[start..].starts_with(prefix) {
                start += prefix.len();
                found = true;
                // Restart the loop once a prefix is removed.
                break;
            }
        }
        if !found {
            break;
        }
    }
    &input[start..]
}

#[repr(u32)]
pub enum AkOpenMode {
    Read = 0x0,
    Write = 0x1,
    WriteOverwrite = 0x2,
    ReadWrite = 0x3,
    ReadEbl = 0x9,
}

/// Tries to find an override for a sound archive entry.
pub fn find_override<'a>(mapping: &'a ArchiveOverrideMapping, input: &str) -> Option<&'a [u16]> {
    let input = strip_prefix(input);
    if input.ends_with(".wem") {
        let wem_path = format!("wem/{input}");
        if let Some(replacement) = get_override(mapping, &wem_path) {
            return Some(replacement);
        }

        // ER stores WEMs at wem/<first two digits of wemID>/wemID.wem so we need to check that
        // location too.
        let folder = input.split_at(2).0;
        let wem_path = format!("wem/{folder}/{input}");
        if let Some(replacement) = get_override(mapping, &wem_path) {
            return Some(replacement);
        }
    } else if let Some(replacement) = get_override(mapping, input) {
        return Some(replacement);
    }

    None
}

fn get_override<'a>(mapping: &'a ArchiveOverrideMapping, input: &str) -> Option<&'a [u16]> {
    for prefix in PREFIXES {
        let prefixed = format!("{prefix}/{input}");
        if let Some((_, replacement)) = mapping.get_override(&prefixed) {
            return Some(replacement);
        }
    }
    None
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use crate::{mapping::ArchiveOverrideMapping, wwise::find_override};

    #[test]
    fn scan_directory_and_overrides() {
        let mut asset_mapping = ArchiveOverrideMapping::default();

        let test_mod_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-data/test-mod");
        asset_mapping.scan_directory(test_mod_dir).unwrap();

        assert!(
            find_override(&asset_mapping, "sd:/init.bnk").is_some(),
            "override for init.bnk was not found"
        );
        assert!(
            find_override(&asset_mapping, "sd:/1000519763.wem").is_some(),
            "override for sd:/1000519763.wem not found"
        );
        assert!(
            find_override(&asset_mapping, "sd:/485927883.wem").is_some(),
            "override for sd:/485927883.wem not found"
        );
    }
}
