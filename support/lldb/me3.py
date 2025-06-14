from typing import Any

import lldb


def __lldb_init_module(
    debugger: lldb.SBDebugger, internal_dict: dict[str, Any]
) -> None:
    for command in [
        f"command script add -o -f {__name__}.load_wine_modules load-wine-modules",
        'target stop-hook add --one-liner "load-wine-modules"'
    ]: debugger.HandleCommand(command)

def load_wine_modules(
    debugger: lldb.SBDebugger,
    command: str,
    result: lldb.SBCommandReturnObject,
    internal_dict: dict[str, Any]
) -> None:
    target: lldb.SBTarget = debugger.GetSelectedTarget()
    if not target or not target.IsValid():
        result.SetError("Invalid Target. Please attach to a process first.")
        return

    process: lldb.SBProcess = target.GetProcess()
    if not process or not process.IsValid():
        result.SetError("Invalid Process. Please attach to a process first.")
        return

    platform: lldb.SBPlatform = target.GetPlatform()
    if not platform or not platform.IsValid():
        result.SetError("Could not get a valid Platform from the target.")
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
        module_candidates[path].append((region.GetRegionBase(), region.GetRegionEnd()))

    for path, regions in module_candidates.items():
        if not path.endswith(".dll") and not path.endswith(".exe"):
            continue

        existing_module: lldb.SBModule = target.FindModule(lldb.SBFileSpec(path))

        if existing_module.IsValid():
            continue

        spec = lldb.SBModuleSpec()
        spec.SetFileSpec(lldb.SBFileSpec(path))
        module: lldb.SBModule = target.AddModule(spec)
        module_base_address = min(start for start, end in regions)

        if module.IsValid():
            target.SetModuleLoadAddress(module, module_base_address)
            print(f"Loaded module {path} at 0x{module_base_address:x}")
        else:
            result.AppendWarning(f"Could not create module for {path}")
