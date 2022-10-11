use std::mem::size_of;

use itertools::Itertools;
use pelite::{
    pe::msvc::{
        RTTIBaseClassDescriptor, RTTIClassHierarchyDescriptor, RTTICompleteObjectLocator,
        TypeDescriptor,
    },
    pe::{Pe, Rva, Va},
};
use rayon::prelude::{ParallelBridge, ParallelIterator};

use super::name;
use crate::Program;

/// A potential reference to an [RTTICompleteObjectLocator] and its adjacent vftable.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct MaybeRttiData {
    vtable_meta_rva: Rva,
    vtable_rva: Rva,
}

pub struct Class<'a> {
    pub name: String,
    pub vtable: Rva,
    pub col: &'a RTTICompleteObjectLocator,
    pub hierarchy_descriptor: &'a RTTIClassHierarchyDescriptor,
    pub base_classes: &'a [RTTIBaseClassDescriptor],
    pub ty: &'a TypeDescriptor,
}

pub fn find_classes(file: Program<'_>) -> impl ParallelIterator<Item = Class<'_>> + '_ {
    find_rtti_data_candidates(file).filter_map(move |candidate| resolve_class(file, candidate))
}

pub fn resolve_class(file: Program<'_>, candidate: MaybeRttiData) -> Option<Class<'_>> {
    let col: &RTTICompleteObjectLocator = file.derva(candidate.vtable_meta_rva).ok()?;

    // Check if mangled type name is printable
    let ty_name = file.derva_c_str(col.type_descriptor + 16).ok()?.to_string();
    if !ty_name
        .chars()
        .all(|ch| (0x20..=0x7e).contains(&(ch as u8)))
    {
        return None;
    }

    let name = name::demangle(&ty_name)?;
    let ty: &TypeDescriptor = file.derva(col.type_descriptor).ok()?;
    let hierarchy_descriptor: &RTTIClassHierarchyDescriptor =
        file.derva(col.class_descriptor).ok()?;
    let base_classes: &[RTTIBaseClassDescriptor] = file
        .derva_slice(
            hierarchy_descriptor.base_class_array,
            hierarchy_descriptor.num_base_classes as usize,
        )
        .ok()?;

    Some(Class {
        name,
        vtable: candidate.vtable_rva,
        col,
        hierarchy_descriptor,
        base_classes,
        ty,
    })
}

/// Analyze a PE file and return an iterator over every potential reference
/// to an [RTTICompleteObjectLocator].
pub fn find_rtti_data_candidates(
    file: Program<'_>,
) -> impl ParallelIterator<Item = MaybeRttiData> + '_ {
    let text = file
        .section_headers()
        .iter()
        .find(|sec| &sec.Name == b".text\0\0\0")
        .expect("no .text section found");

    let rdata = file
        .section_headers()
        .iter()
        .find(|sec| &sec.Name == b".rdata\0\0")
        .expect("no .rdata section found");

    let text_bounds = text.virtual_range();
    let rdata_bounds = rdata.virtual_range();

    rdata
        .virtual_range()
        .step_by(size_of::<Va>())
        .tuple_windows()
        .par_bridge()
        .filter_map(move |(vtable_meta_ptr_rva, vtable_rva)| {
            let vtable_meta_rva = file
                .derva(vtable_meta_ptr_rva)
                .and_then(|va| file.va_to_rva(*va))
                .ok()?;

            let vtable_entry_rva = file
                .derva(vtable_rva)
                .and_then(|va| file.va_to_rva(*va))
                .ok()?;

            if rdata_bounds.contains(&vtable_meta_rva) && text_bounds.contains(&vtable_entry_rva) {
                let _: &RTTICompleteObjectLocator = file.derva(vtable_meta_rva).ok()?;

                Some(MaybeRttiData {
                    vtable_meta_rva,
                    vtable_rva,
                })
            } else {
                None
            }
        })
}
