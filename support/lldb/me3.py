import os
from typing import Any

import lldb


def __lldb_init_module(
    debugger: lldb.SBDebugger, internal_dict: dict[str, Any]
) -> None:
    for command in [
        f"command script add -o -f {__name__}.load_wine_modules load-wine-modules",
        f"command script add -o -f {__name__}.dump pe-dump",
        'target stop-hook add --one-liner "load-wine-modules"'
    ]:
        debugger.HandleCommand(command)


def attached_state(debugger: lldb.SBDebugger) -> tuple[lldb.SBTarget, lldb.SBProcess, lldb.SBPlatform]:
    target: lldb.SBTarget = debugger.GetSelectedTarget()
    if not target or not target.IsValid():
        raise Exception("Invalid Target. Please attach to a process first.")

    process: lldb.SBProcess = target.GetProcess()
    if not process or not process.IsValid():
        raise Exception("Invalid Process. Please attach to a process first.")

    platform: lldb.SBPlatform = target.GetPlatform()
    if not platform or not platform.IsValid():
        raise Exception("Could not get a valid Platform from the target.")

    return (target, process, platform)


def dump(
    debugger: lldb.SBDebugger,
    args: str,
    result: lldb.SBCommandReturnObject,
    internal_dict: dict[str, Any]
) -> None:
    try:
        target, process, platform = attached_state(debugger)
    except Exception as e:
        result.SetError(str(e))
        return

    module_name, output_path = args.split(' ', 2)
    memory_regions = process.GetMemoryRegions()
    start_addr = None
    end_addr = None

    region = lldb.SBMemoryRegionInfo()
    for i in range(memory_regions.GetSize()):
        if not memory_regions.GetMemoryRegionAtIndex(i, region):
            continue

        path: str = region.GetName()
        if not path:
            continue

        if path.endswith(module_name):
            if start_addr is None:
                start_addr = region.GetRegionBase()

            if end_addr is None:
                end_addr = region.GetRegionEnd()

            start_addr = min(start_addr, region.GetRegionBase())
            end_addr = max(end_addr, region.GetRegionEnd())

    result.Print(f"Dumping {module_name}@0x{start_addr:x}-{end_addr:x}")

    bytes_written = 0
    error = lldb.SBError()

    with open(output_path, 'wb') as f:
        addr = start_addr
        while addr < end_addr:
            read_size = min(1024, end_addr - addr)

            mem = target.ReadMemory(lldb.SBAddress(
                addr, target), read_size, error)

            if error.Success():
                f.write(mem)
            else:
                result.Print(str(error))
                return

            bytes_written += read_size
            addr += read_size

    result.Print(f"Success! Wrote {bytes_written} bytes.")
    import pefile
    pe = pefile.PE(output_path)

    for section in pe.sections:
        section.SizeOfRawData = section.Misc_VirtualSize
        section.PointerToRawData = section.VirtualAddress

    pe.write(output_path)


def load_wine_modules(
    debugger: lldb.SBDebugger,
    command: str,
    result: lldb.SBCommandReturnObject,
    internal_dict: dict[str, Any]
) -> None:
    try:
        target, process, platform = attached_state(debugger)
    except Exception as e:
        result.SetError(str(e))
        return

    memory_regions = process.GetMemoryRegions()
    module_candidates: dict[str, list[tuple[int, int]]] = {}

    region = lldb.SBMemoryRegionInfo()
    for i in range(memory_regions.GetSize()):
        if not memory_regions.GetMemoryRegionAtIndex(i, region):
            continue

        path = region.GetName()
        if not path:
            continue

        if path not in module_candidates:
            module_candidates[path] = []
        module_candidates[path].append(
            (region.GetRegionBase(), region.GetRegionEnd()))

    for path, regions in module_candidates.items():
        existing_module: lldb.SBModule = target.FindModule(
            lldb.SBFileSpec(path))

        if existing_module.IsValid():
            continue

        file_spec = lldb.SBFileSpec(path)
        spec = lldb.SBModuleSpec()
        spec.SetFileSpec(file_spec)
        spec.SetTriple("x86_64-pc-windows-msvc")

        module: lldb.SBModule = target.AddModule(spec)
        module.SetPlatformFileSpec(file_spec)
        module_base_address = min(start for start, end in regions)

        if module.IsValid():
            base_addr = module_base_address
            target.SetModuleLoadAddress(module, base_addr)
            print(f"Loaded module {path} at 0x{base_addr:x}")
        else:
            result.AppendWarning(f"Could not create module for {path}")
