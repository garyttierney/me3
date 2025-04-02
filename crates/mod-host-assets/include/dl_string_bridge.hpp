#pragma once

#include <string>
#include <rust/cxx.h>
#include "dl_allocator.hpp"

#if defined(_ITERATOR_DEBUG_LEVEL) && _ITERATOR_DEBUG_LEVEL > 0
#error "_ITERATOR_DEBUG_LEVEL" must be defined as "0" for STL containers to be compatible with the ELDEN RING ABI.
#endif

template <typename CharT>
struct DLBasicString {
    mutable std::basic_string<CharT, std::char_traits<CharT>, DLKR::DLAllocatorAdapter<CharT>> inner;
    bool _unk0x28;
};

using DLWString = DLBasicString<char16_t>;

rust::string get_dlwstring_contents(const DLWString& str) noexcept {
    return rust::string(str.inner.data(), str.inner.length());
}

void set_dlwstring_contents(const DLWString& str, rust::slice<const uint16_t> contents) noexcept {
    const char16_t* first = reinterpret_cast<const char16_t*>(contents.data());
    size_t length = contents.size();
    str.inner.assign(first, length);
}
