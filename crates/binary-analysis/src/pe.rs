use std::mem::MaybeUninit;

use pelite::pe::{headers::SectionHeader, Pe};

pub fn section<'a, P, S>(program: P, name: S) -> Result<&'a SectionHeader, S>
where
    P: Pe<'a>,
    S: AsRef<[u8]>,
{
    program.section_headers().by_name(&name).ok_or(name)
}

pub fn sections<'a, P, S, const N: usize>(
    program: P,
    names: [S; N],
) -> Result<[&'a SectionHeader; N], S>
where
    P: Pe<'a>,
    S: AsRef<[u8]>,
{
    let sections = program.section_headers();

    let mut result = [MaybeUninit::uninit(); N];

    for (i, name) in names.into_iter().enumerate() {
        result[i].write(sections.by_name(&name).ok_or(name)?);
    }

    // SAFETY: all elements have been initialized or the function returned early.
    unsafe { Ok(result.map(|e| e.assume_init())) }
}
