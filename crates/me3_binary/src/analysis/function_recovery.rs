use std::cmp::Ordering;

use itertools::Itertools;
use pelite::pe::{exception::UnwindInfo, Pe, Rva};

use crate::Program;

// TODO: a function should contain a set of address ranges containing representing
// each basic block, instead of just an entrypoint and size.
pub struct Function<'a> {
    pub entry: Rva,
    pub size: u32,
    pub unwind_info: Option<UnwindInfo<'a, Program<'a>>>,
}

/// Given a [Program], attempt to identify and locate all functions
/// from an authoritative source (e.g. unwind tables and class vftables).
pub fn find_functions(program: Program) -> impl Iterator<Item = Function> + '_ {
    // TODO: merge with functions from vftables.
    program
        .exception()
        .ok()
        .into_iter()
        .flat_map(|exception_table| exception_table.functions())
        .map(|function| Function {
            entry: function.image().BeginAddress,
            size: function.image().EndAddress - function.image().BeginAddress,
            unwind_info: function.unwind_info().ok(),
        })
        .sorted_by_key(|func| func.entry)
}

/// Given a [Program], identify the function containing code at `address`.
pub fn find_function_containing(program: Program, address: Rva) -> Option<Function> {
    let mut functions: Vec<Function> = find_functions(program).collect();

    functions
        .binary_search_by(|f| {
            if address >= f.entry + f.size {
                Ordering::Greater
            } else if address < f.entry {
                Ordering::Less
            } else {
                Ordering::Equal
            }
        })
        .ok()
        .map(move |index| functions.swap_remove(index))
}
