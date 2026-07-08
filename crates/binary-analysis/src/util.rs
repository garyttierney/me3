use pelite::pe::{Pe, Rva};
use rayon::iter::{ParallelBridge, ParallelIterator};

/// Find rip-relative LEA instructions that target the given RVA.
pub fn lea_refs<'a, P: Pe<'a>>(program: P, target: Rva) -> pelite::Result<Vec<Rva>> {
    // Manually parallelize over 64KiB blocks.
    //
    // Benchmarking shows that perf doesn't degrade unless blocks are very small
    // (<4096 bytes) or so large that available parallelism is underused.
    const PAR_CHUNK_SIZE: usize = 0x10000;

    let mut results = Vec::new();
    for section in program.section_headers() {
        if section.name_bytes() != b".text" {
            continue;
        }

        let section_rva = section.VirtualAddress;
        let target_rel = target.wrapping_sub(section_rva);

        let section_slice = program.get_section_bytes(section)?;
        // We scan for the LEA opcode, which is after the REX prefix, so start at 1.
        // The entire instruction is 7 bytes, so ignore the last 6 byte positions.
        let scan_slice = &section_slice[1..section_slice.len() - 6];
        // We can't use `chunks(PAR_CHUNK_USIZE)` here, since we need the index too.
        let par_split = (0..scan_slice.len()).step_by(PAR_CHUNK_SIZE).par_bridge();
        let section_results = par_split
            .flat_map_iter(|start| {
                let end = scan_slice.len().min(start + PAR_CHUNK_SIZE);
                // Use memchr to search for the LEA opcode (0x8d).
                memchr::memchr_iter(0x8d, &scan_slice[start..end]).filter_map(move |i| {
                    let lea_candidate = start + i;
                    let instr_start = lea_candidate;
                    let modrm = lea_candidate + 2;
                    let disp_start = lea_candidate + 3;
                    let instr_end = lea_candidate + 7;

                    // Check REX byte, masking REX.B
                    if (section_slice[instr_start] & 0b1111_1011) != 0x48 {
                        return None;
                    }
                    // Check Mod.RM byte, masking REG
                    if (section_slice[modrm] & 0b1100_0111) != 5 {
                        return None;
                    }
                    // Check displacement
                    let disp_slice = &section_slice[disp_start..instr_end];
                    let disp = i32::from_le_bytes(disp_slice.try_into().unwrap());
                    ((instr_end as Rva).wrapping_add_signed(disp) == target_rel)
                        .then(|| section_rva + instr_start as Rva)
                })
            })
            .collect_vec_list();

        results.extend(section_results.into_iter().flatten());
    }

    Ok(results)
}
