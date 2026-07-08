use pelite::{
    pe::{Pe, Ptr, Rva, Va},
    Pod,
};
use thiserror::Error;

use crate::pe::sections;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RuntimeClassEntry {
    pub type_id: Va,
    pub class: Ptr<()>,
}

unsafe impl Pod for RuntimeClassEntry {}

#[derive(Debug, Error)]
pub enum RuntimeClassesError {
    #[error(transparent)]
    Pelite(#[from] pelite::Error),
    #[error("PE section \"{0}\" is missing")]
    Section(&'static str),
    #[error("RuntimeClass registry not found")]
    NotFound,
}

/// Scan an initialized game's .data section, returning the DLRuntimeClass registry.
///
/// This is a slice of [`RuntimeClassEntry`], sorted by type ID.
pub fn runtime_classes<'a, P: Pe<'a>>(
    program: P,
) -> Result<&'a [RuntimeClassEntry], RuntimeClassesError> {
    let [data, rdata] =
        sections(program, [".data", ".rdata"]).map_err(RuntimeClassesError::Section)?;

    let data_virtual_range = data.virtual_range();
    let rdata_virtual_range = rdata.virtual_range();

    let data_range =
        program.rva_to_va(data_virtual_range.start)?..program.rva_to_va(data_virtual_range.end)?;
    let rdata_range = program.rva_to_va(rdata_virtual_range.start)?
        ..program.rva_to_va(rdata_virtual_range.end)?;

    let mut best_run = 0;
    let mut best_start = None;

    let mut current_run = 0;
    let mut current_start = 0;
    let mut last_type_id = 0;

    const ENTRY_SIZE: usize = size_of::<RuntimeClassEntry>();

    for rva in data_virtual_range.step_by(ENTRY_SIZE) {
        let type_id: Ptr<u8> = *program.derva(rva)?;
        let rtc_ptr: Ptr<Va> = *program.derva(rva + size_of::<Va>() as Rva)?;

        let is_valid = type_id.into_raw() > last_type_id
            && data_range.contains(&type_id.into_raw())
            && data_range.contains(&rtc_ptr.into_raw())
            && program.deref(type_id) == Ok(&0)
            && program
                .deref(rtc_ptr)
                .is_ok_and(|vmt| rdata_range.contains(vmt));

        if is_valid {
            current_run += 1;
            last_type_id = type_id.into_raw();
            if current_run > best_run {
                best_start = Some(current_start);
                best_run = current_run;
            }
        } else {
            current_run = 0;
            last_type_id = 0;
            current_start = rva + ENTRY_SIZE as Rva;
        }
    }

    // To guard against the registry structure being changed in the future,
    // expect at least 512 runtime classes. All have over 4000, so
    // this seems reasonable.
    let registry_rva = best_start
        .filter(|_| best_run >= 512)
        .ok_or(RuntimeClassesError::NotFound)?;

    Ok(program.derva_slice(registry_rva, best_run)?)
}
