# Reversing Arxan (now GuardIT)
This article focuses on the reverse engineering and declawing of the Arxan anti-debug and tamper protection software as it is used in FROMSOFTWARE games. It assumes basic x86 assembly and low-level reverse engineering knowledge.

## Introduction
If you've ever tried to attach a debugger to a FROMSOFTWARE game, you probably experienced random crashes. Likewise, modders that need to hook or patch native game code have likely experienced crashes and/or their patches being reverted by the game after a random amount of time.

The source of these problems is an anti-debug and tamper protection product now called GuardIT. However, it is better known in the community by its older name, Arxan, which I will use to refer to it throughout this article. FROMSOFTWARE has applied it to all of their PC releases since Dark Souls II: SOTFS, excluding Sekiro. Some of its features include:

- Instruction mutations and control flow obfuscation to confuse decompilers and make reverse engineering harder
- Encryption of sensitive functions at rest, decrypting them ephemerally when they are being executed
- A varied suite of anti-debug checks
- Integrity checks on functions marked as sensitive by the game developer, with the ability to a combination of the following when tampering is detected:
    - Silently writing flags to a buffer that the game developer can read to integrate with their anti-cheat solution. FROMSOFTWARE uses this to ban players that try to tamper with the game's code while playing online.
    - Crashing the game by corrupting the stack or control flow in a way that is difficult to debug
    - Repairing the function's code

Running into these protections when reverse engineering the game or when modding it can be a serious hassle. In fact, Dark Souls III support in me3 was, for a long time, blocked by the need to hook a function whose code integrity is checked by Arxan. As such, a way to fully disable it and ensure that no Arxan logic is running would be highly desirable. This blog will cover my journey through reversing how Arxan inserts its code into executables, how it protects itself from tampering attempts and how its function encryption mechanism works, culminating in the release of [dearxan](https://crates.io/crates/dearxan), a Rust crate that is able to neuter it at runtime in all FROMSOFTWARE games.

## Prior Art
There has been a few attempts to counter Arxan's protections by the Souls modding and cheating community. Most focus on a specific feature, such as code integrity checks, code restoration or anti-debug, support a limited set of games, or require manually finding offsets to problematic Arxan code (which has to be done for every supported game version).

#### DS3 and DSR "anti cheat bypasses"
Dark Souls III and Dark Souls Remastered had a very active online cheating community. Many cheats required hooking game functions that are protected by Arxan. Detection of this tampering while playing online would lead to bans, so some have taken a crack at semi-manually patching out most of the integrity checks. The resulting "bypasses" were kept private and distributed among small groups of cheaters before inevitably getting leaked to more and more people. They were also used by early community anti cheats such as the [DS3 PvP Watchdog](https://www.nexusmods.com/darksouls3/mods/352).

#### [MetalCrow](https://github.com/metal-crow)'s DS1 Overhaul anti-anti cheat
The [Dark Souls 1 Overhaul](https://github.com/metal-crow/Dark-Souls-1-Overhaul) mod is meant to dramatically improve the online PvP experience in DarkSoulsRemastered. To prevent users being banned due to the code patches made by the mod, code integrity checks where manually found so they can be patched out by the mod ([source](https://github.com/metal-crow/Dark-Souls-1-Overhaul/blob/master/OverhaulDLL/src/AntiAntiCheat.cpp)). 

#### Yui's anti code restoration patches
[Yui](https://github.com/LukeYui)'s [Seamless Co-op](https://www.nexusmods.com/eldenring/mods/510) mods require hundreds of hooks to function, many of which target code that is protected by Arxan's anti-tamper code restoration feature. She found that since Elden Ring, these routines are (almost?) all triggered from timed checks in regular game code, most likely for performance reasons. The code determining if it's time to run an individual check looks like this:

<a name="timed_restoration_check"></a>
```c linenums="1"
if (TIME_LEFT == 0.0) {
    TIME_LEFT = (float)get_random_delay_seconds(CHECK_FLAG);
}
else {
    TIME_LEFT -= 0.016666; // one 60FPS frame
    if (TIME_LEFT <= 0.0) {
        arxan_code_restoration_check();
    }
}
```

Or, in assembly:
```nasm linenums="1" hl_lines="14"
MOVSS      XMM0,dword ptr [TIME_LEFT]
UCOMISS    XMM0,XMM6
JP         no_timer_reset
JNZ        no_timer_reset
MOV        ECX,0x10cd ; CHECK_FLAG
CALL       get_random_delay_seconds
MOVSS      dword ptr [TIME_LEFT],XMM0
JMP        check_end

no_timer_reset:
ADDSS      XMM0,dword ptr [NEGATIVE_ONE_OVER_SIXTY] 
MOVSS      dword ptr [TIME_LEFT],XMM0
COMISS     XMM6,XMM0
JC         check_end
LEA        RDX,[TIME_LEFT]
MOV        RCX,RBX
CALL       arxan_code_restoration_check

check_end:
```

It happens to be very easy to scan for this instruction pattern and replace the final `JC` above with an unconditional jump to skip the check ([example implementation](https://github.com/tremwil/param_field_mapper/blob/master/src/arxan_disabler.cpp)).

Given its simplicity and ease of implementation, I would still recommend this technique if your mod targets Elden Ring or more recent FROMSOFTWARE games and all you need is to make sure your hooks aren't tampered with.

#### Dasaav's [ProDebug](https://github.com/Dasaav-dsv/ProDebug)
This DLL mod by [Dasaav](https://github.com/Dasaav-dsv) uses pattern scanning to identify timers in the game code that trigger some of Arxan's anti debug checks. In Elden Ring and Armored Core VI, this seems to be the only place from which the anti-debug routines run, making the technique sufficient to disable (almost?) all anti-debug features.

#### Maurice Heumann's BO3 reversing work
[This excellent blog post](https://momo5502.com/posts/2022-11-17-reverse-engineering-integrity-checks-in-black-ops-3/) by [Maurice Heumann](https://github.com/momo5502/) showcases patching out Arxan integrity checks in Call of Duty: Black Ops III. Since I was not aware of this before having come up with my own technique for disabling Arxan, I did not investigate his method and whether it works in FROMSOFTWARE titles. Note that it onlu addresses the integrity checks, and not the anti-debug routines or runtime function encryption.

## Motivation
I began this project a few months ago at the request of [Yui](https://github.com/LukeYui), when she was working on porting her [Seamless Co-op](https://www.nexusmods.com/eldenring/mods/510) mod to Dark Souls Remastered. The technique [outlined above](#yuis-anti-code-restoration-patches) for preventing Arxan to restore modified code regions does not work in DSR, as the code restoration checks are inserted in random functions as opposed to being put into dedicated timer routines. Futhermore, many code integrity checks immediately crash the game, so patching the restoration step out would not be sufficient.

Thus, the mod was blocked on being able to disable Arxan code integrity checks in general. I knew of [MetalCrow's prior work](#metalcrows-ds1-overhaul-anti-anti-cheat) on this, but did not want to rely on a hardcoded list of check addresses. I thus started from scratch by inspecting the structure of the code restoration routines that the Elden Ring specific pattern could find.

## The Arxan Stub

### General Structure

Looking at `arxan_code_restoration_check` in one of [the timer-based code restoration check pattern](#timed_restoration_check) matches, we see the following (ER 1.16.0 @ `145c62312`):

```nasm linenums="1" hl_lines="3 7 13 15"
MOV     qword ptr [RSP + -0x8],RBX
LEA     RSP,[RSP + -0x8]
LEA     RBX,[LAB_14040e11c]
MOV     qword ptr [RSP + -0x8],RAX
LEA     RSP,[RSP + -0x8]
MOV     RAX,qword ptr [RSP + 0x8]
MOV     qword ptr [RSP + 0x8],RBX=>LAB_14040e11c
PUSH    RAX
POP     RBX
MOV     RAX,qword ptr [RSP]
LEA     RSP,[RSP + 0x8]
PUSH    R12
LEA     R12,[LAB_1459bfddb]
PUSH    qword ptr [RSP]
MOV     qword ptr [RSP + 0x8],R12=>LAB_1459bfddb
POP     R12
RET     =>LAB_1459bfddb
```

This is heavily obfuscated using Arxan's instruction substitution based obfuscation engine. However, by keeping track of the stack pointer and the two referenced addresses (lines 3 and 13) and their uses, we can see that we end up writing `LAB_14040e11c` to `RSP-8` and jumping to `LAB_1459bfddb`. In fact, going through the operations symbolically we can drastically simplify the code:

```nasm
MOV     [RSP-0x08], LAB_14040e11c
MOV     [RSP-0x10], R12
MOV     [RSP-0x18], R12
LEA     RSP, [RSP - 8]
JMP     LAB_1459bfddb
```

Ignoring the `R12` stack clobbers, note that we're pushing an address on the stack and jumping. This is essentially what a call instruction does! In fact, all of these instructions are effectively obfuscation for

```nasm
CALL    LAB_1459bfddb
JMP     LAB_14040e11c
```

This general pattern (move the return and call address to the stack, then jump using a `RET`) turns out to be great for identifying Arxan obfuscated calls.

The return address, `LAB_14040e11c`, points to a seemingly random instruction which turns out to be in the middle of a function related to [Havok](https://en.wikipedia.org/wiki/Havok_(software)) script execution. Jumping here is not valid and would certainly crash the program in a hard-to-debug way.

Looking at the call address, `LAB_1459bfddb`, we find another obfuscated call: 

```nasm linenums="1" hl_lines="3 6"
LEA     RSP,[RSP + -0x8]
MOV     qword ptr [RSP],R14
LEA     R14,[LAB_141f3e1b3]
XCHG    qword ptr [RSP],R14
PUSH    R9
LEA     R9,[LAB_144ff7fde]
PUSH    RAX
MOV     RAX,qword ptr [RSP + 0x8]
MOV     qword ptr [RSP + 0x8],R9=>LAB_144ff7fde
MOV     R9,RAX
POP     RAX
RET     =>LAB_144ff7fde
```

Again, the return address (line 3) points to a random instruction in the STL code for `std::error_category::equivalent`. The call address points to a third obfuscated call:

```nasm linenums="1" hl_lines="3 9"
LEA     RSP,[RSP + -0x8]
MOV     qword ptr [RSP],R11
LEA     R11,[LAB_145c97b12]
PUSH    qword ptr [RSP]
MOV     qword ptr [RSP + 0x8],R11=>LAB_145c97b12
LEA     RSP,[RSP + 0x8]
MOV     R11,qword ptr [RSP + -0x8]
PUSH    R11
LEA     R11,[LAB_14515620d]
PUSH    qword ptr [RSP]
MOV     qword ptr [RSP + 0x8],R11=>LAB_14515620d
POP     R11
RET     =>LAB_14515620d
```

The return address for this one, `LAB_145c97b12`, was more difficult to identify, as the code it points to is extremely obfuscated. However, upon a closer look it again seems to be in the middle of a regular game function. The call address (`LAB_14515620d`), however, now points to something more interesting:

```nasm linenums="1"
PUSHFQ
LEA     RSP,[RSP + -0x8]
MOV     qword ptr [RSP],RAX
MOV     qword ptr [RSP + -0x8],RCX
LEA     RSP,[RSP + -0x8]
MOV     qword ptr [RSP + -0x8],R8
LEA     RSP,[RSP + -0x8]
MOV     qword ptr [RSP + -0x8],RDX
LEA     RSP,[RSP + -0x8]
PUSH    0x10
TEST    RSP,0xf
LEA     RSP,[RSP + -0x8]
MOV     qword ptr [RSP],RSI
MOV     RSI,LAB_1404e6f39
PUSH    qword ptr [RSP]
MOV     qword ptr [RSP + 0x8],RSI=>LAB_1404e6f39
LEA     RSP,[RSP + 0x8]
MOV     RSI,qword ptr [RSP + -0x8]
MOV     qword ptr [RSP + -0x8],RBX
LEA     RSP,[RSP + -0x8]
LEA     RSP,[RSP + -0x8]
MOV     qword ptr [RSP],RAX
MOV     RBX,qword ptr [RSP + 0x10]
MOV     RAX,LAB_140bd28e4
CMOVNZ  RBX,RAX
MOV     qword ptr [RSP + 0x10],RBX
MOV     RAX,qword ptr [RSP]
LEA     RSP,[RSP + 0x8]
MOV     RBX,qword ptr [RSP]
LEA     RSP,[RSP + 0x8]
LEA     RSP,[RSP + 0x8]
JMP     qword ptr [RSP + -0x8]
```

This block is heavily obfuscated, but again, we can painstakingly go through the instructions to see that it's mostly an obfuscated conditional jump:

```nasm linenums="1"
PUSHFQ
PUSH    RAX
PUSH    RCX
PUSH    R8
PUSH    RDX
PUSH    0x10
TEST    RSP,0xf
JNZ     LAB_140bd28e4
JMP     LAB_1404e6f39
```

The first part of this looks very much like a partial context save. Inspecting the branches further and deobfuscating leaves us with the following graph, confirming this suspicion:

<a name="stub_cfg">
<pre class="mermaid">
stateDiagram-v2
    classDef text text-align: left
    state &quot;&lt;i&gt;context_save:&lt;/i&gt;&lt;code&gt;
        PUSHFQ
        PUSH RAX
        PUSH RCX
        PUSH R8
        PUSH RDX
        PUSH 0x10
        TEST RSP,0xf
        JNZ rsp_unaligned
        JMP rsp_aligned
    &lt;/code&gt;
    &quot; as context_save

    state &quot;&lt;i&gt;rsp_unaligned:&lt;/i&gt;&lt;code&gt;
        SUB RSP,8
        CALL FUN_1458d2c3d
        JMP context_pop
    &lt;/code&gt;
    &quot; as rsp_unaligned

    state &quot;&lt;i&gt;rsp_aligned:&lt;/i&gt;&lt;code&gt;
        PUSH 0x18
        JMP rsp_unaligned
    &lt;/code&gt;
    &quot; as rsp_aligned

    state &quot;&lt;i&gt;context_pop:&lt;/i&gt;&lt;code&gt;
        ADD RSP, [RSP+8]
        POP RDX
        POP R8
        POP RCX
        POP RAX
        POPFQ
        RET
    &lt;/code&gt;
    &quot; as context_pop

    context_save:::text --&gt; rsp_unaligned: NZ
    context_save --&gt; rsp_aligned:::text: Z 
    rsp_aligned --&gt; rsp_unaligned:::text
    rsp_unaligned --&gt; context_pop:::text
</pre>

Tracking the stack carefully, we can see that the `PUSH 0x10` and `PUSH 0x18` instructions serve to restore the original alignment of the stack through the `ADD RSP, [RSP+8]` instruction in `context_pop`. While I've never seen this stack alignment technique before (usually it's done branchlessly by saving `RSP` in a temporary register and simply clear the low 4 bits of RSP), this is a very common pattern for instrumentation: save the existing CPU state on the stack, call the instrumentation routine (in this case, the call to `FUN_1458d2c3d` which presumably handles the code restoration logic), and restore the original state.

However, there is a problem here: because of the first three obfuscated calls with bogus return addresses, the final `RET` will redirect execution to the middle of an unrelated function and inevitably crash the game. Thus, I guessed that there must be something in `FUN_1458d2c3d` that writes over these "fake" return addresses. I tried to manually follow the logic of this function, but the obfuscation successfully prevented Ghidra and IDA decompilers / CFG viewers from working, and proved too annoying to go through manually. 

### Identification
Instead of immediately coming up with a better analysis workflow or deobfuscation strategy, I wondered if I could find simpler "Arxan stubs". From the above CFG, it seemed plausible that this was *the* way Arxan inserted logic into existing game code, so I started pattern scanning.

This is where Arxan's use of the `TEST RSP, 0xf` instruction comes back to haunt them. As I've said before, it's a very unusual way to align the stack, and I can't think of any other scenarios where such an instruction would be emitted. Sure enough, a search for the bytes `48 f7 c4 0f 00 00 00` gives 1598 matches. Going through a few dozen, they all follow the same context save/restore pattern! With more examples, it becomes evident that Arxan developers did try to make it harder to find these stubs:

- The order in which registers are pushed and popped to the stack is randomized.
- The number of registers saved depends on the stub. Sometimes it's a few GPRs, sometimes it's all of them, sometimes it's also XMM registers.
- Even when the same amount of registers are pushed, there can be random "gaps" in the stack-saved context.
- Basic blocks are regularly split into smaller blocks of few instructions, likely to defend against pattern scans.
- Surprisingly, many of them are not obfuscated at all, and look very much like the [deobfuscated CFG shown above](#stub_cfg). The non-obfuscated ones lack the 3 obfuscated "fake" calls leading to the context save.

### Return Gadgets

Now that I had a way to find all stubs, I returned to Dark Souls Remastered to continue my analysis there, as it was the game I needed to support first and foremost. The `TEST RSP, 0xf` pattern matched 2976 times in that game (which happens to be higher than all other FROMSOFTWARE games). To find what was writing over the bogus return addresses, I used the Rust bindings for the [Unicorn](https://www.unicorn-engine.org/) emulator to step through a stub starting at the `TEST RSP, 0xf` instruction while logging instructions that wrote to a stack address above the original RSP, which lead me to blocks like this (high stack writes highlighted):

```nasm linenums="1" hl_lines="4 9 14"
MOV     RAX,qword ptr [LAB_14009f568]       = 0x32
MOV     RDX,qword ptr [RBP + 0x50]
MOV     RCX,qword ptr [PTR_LAB_140095c28]   = 0x1400e0f5a
MOV     qword ptr [RDX + RAX*0x8],RCX=>LAB_1400e0f5a

MOV     RAX,qword ptr [DAT_140663c78]       = 0x31
MOV     RDX,qword ptr [RBP + 0x50]
MOV     RCX,qword ptr [PTR_LAB_141056011]   = 0x140997167
MOV     qword ptr [RDX + RAX*0x8],RCX=>LAB_140997167

MOV     RAX,qword ptr [DAT_140a6ccf7]       = 0x30
MOV     RDX,qword ptr [RBP + 0x50]
MOV     RCX,qword ptr [PTR_LAB_14012f813]   = 0x14026c7d5
MOV     qword ptr [RDX + RAX*0x8],RCX=>LAB_14026c7d5
```

Inspecting `LAB_1400e0f5a`, we find a `JMP` instruction to the beginning of another obfuscated Arxan stub (i.e. the 3 repeated obfuscated calls with bogus return addresses). The other two gadgets, `LAB_140997167` and `LAB_14026c7d5`, are simply obfuscated return instructions, such as `LEA RSP, [RSP + 8]; JMP qword ptr [RSP - 8]`. 

Letting the emulator execute shows that indeed, once the context restoration part of the stub finishes, we jump from one return gadget to the next, ending up at the true exit address of `LAB_1400e0f5a`. 