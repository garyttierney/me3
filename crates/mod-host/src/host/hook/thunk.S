.global thunk_prototype_start
.global thunk_prototype_lea
.global thunk_prototype_jmp
thunk_prototype_start:
lea rax,[rip+thunk_prototype_start]
thunk_prototype_lea:
mov gs:[0x28],rax
jmp [rip+thunk_prototype_start]
thunk_prototype_jmp:
