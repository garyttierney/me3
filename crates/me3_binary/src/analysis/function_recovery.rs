use pelite::pe::{exception::UnwindInfo, Pe, Rva};

use crate::Program;

pub struct Function<'a> {
    entry: Rva,
    unwind_info: Option<UnwindInfo<'a, Program<'a>>>,
}

/// Given a [Program], attempt to identify and locate all functions
/// from an authoritative source (e.g. unwind tables and class vftables).
pub fn find_functions(program: Program<'_>) -> impl Iterator<Item = Function> + '_ {
    // TODO: merge with functions from vftables.
    program
        .exception()
        .ok()
        .into_iter()
        .flat_map(|exception_table| exception_table.functions())
        .map(|function| Function {
            entry: function.image().BeginAddress,
            unwind_info: function.unwind_info().ok(),
        })
}
