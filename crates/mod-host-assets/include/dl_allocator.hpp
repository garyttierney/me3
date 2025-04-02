#pragma once

#include <cstdint>
#include <type_traits>

namespace DLKR {
class DLAllocationInterface {
public:
    virtual ~DLAllocationInterface() = default;
    virtual uint32_t GetAllocatorId() = 0;
    virtual int32_t _unk0x10() = 0;
    virtual uint32_t& GetHeapFlags(uint32_t& out) = 0;
    virtual uint64_t GetHeapCapacity() = 0;
    virtual uint64_t GetHeapSize() = 0;
    virtual uint64_t GetBackingHeapCapacity() = 0;
    virtual uint64_t GetAllocationCount() = 0;
    virtual uint64_t GetSizeOfAllocation(void* pAllocation) = 0;
    virtual void* AllocateMemory(uint64_t sizeBytes) = 0;
    virtual void* AllocateAlignedMemory(uint64_t sizeBytes, uint64_t alignment) = 0;
    virtual void* ReallocateMemory(void* pAllocation, uint64_t sizeBytes) = 0;
    virtual void* ReallocateAlignedMemory(void* pAllocation, uint64_t sizeBytes, uint64_t alignment) = 0;
    virtual void FreeMemory(void* pAllocation) = 0;
};

template <typename T>
class DLAllocatorAdapter {
public:
    using value_type = T;
    using size_type = uint64_t;
    using difference_type = int64_t;

    using propagate_on_container_move_assignment = std::true_type;
    using is_always_equal = std::false_type;

    template <typename U>
    DLAllocatorAdapter(const DLAllocatorAdapter<U>& other) noexcept : allocator(other.allocator) {}

    T* allocate(size_type count) {
        return reinterpret_cast<T*>(allocator.AllocateAlignedMemory(count * sizeof(T), alignof(T)));
    }

    void deallocate(T* pAllocation, size_type count = 0) {
        allocator.FreeMemory(reinterpret_cast<void*>(pAllocation));
    }

    template <typename T1, typename T2>
    friend bool operator==(const DLAllocatorAdapter<T1>& lhs, const DLAllocatorAdapter<T2>& rhs) noexcept {
        return &lhs.allocator == &rhs.allocator;
    }

private:
    DLAllocationInterface& allocator;
};

}
