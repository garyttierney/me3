use iced_x86::{Decoder, DecoderOptions, Instruction};
use pelite::pe::{Pe, Rva};

use crate::Program;

pub struct InstructionIterator<'a> {
    decoder: Decoder<'a>,
}

impl<'a> Iterator for InstructionIterator<'a> {
    type Item = Instruction;

    fn next(&mut self) -> Option<Self::Item> {
        if self.decoder.can_decode() {
            Some(self.decoder.decode())
        } else {
            None
        }
    }
}

pub fn disassemble(program: Program, address: Rva) -> impl Iterator<Item = Instruction> + '_ {
    let decoder = Decoder::with_ip(
        64,
        program
            .slice_bytes(address)
            .expect("unable to get bytes at code address"),
        program
            .rva_to_va(address)
            .expect("couldn't transform RVA to VA"),
        DecoderOptions::NONE,
    );

    InstructionIterator { decoder }
}
