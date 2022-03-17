//! Easy to use, high performance memory manager for Vulkan.

use bitflags::bitflags;

pub mod ffi;
use ash::prelude::VkResult;
use ash::vk;
use std::mem;

/* #region BITFLAGS & ENUMS */

bitflags! {
    /// Flags for configuring `Allocator` construction.
    pub struct AllocatorCreateFlags: u32 {
        /// No allocator configuration other than defaults.
        const NONE = 0x0000_0000;

        /// Allocator and all objects created from it will not be synchronized internally,
        /// so you must guarantee they are used from only one thread at a time or synchronized
        /// externally by you. Using this flag may increase performance because internal
        /// mutexes are not used.
        const EXTERNALLY_SYNCHRONIZED = 0x0000_0001;

        /// Enables usage of `VK_KHR_dedicated_allocation` extension.
        ///
        /// Using this extenion will automatically allocate dedicated blocks of memory for
        /// some buffers and images instead of suballocating place for them out of bigger
        /// memory blocks (as if you explicitly used `AllocationCreateFlags::DEDICATED_MEMORY` flag) when it is
        /// recommended by the driver. It may improve performance on some GPUs.
        ///
        /// You may set this flag only if you found out that following device extensions are
        /// supported, you enabled them while creating Vulkan device passed as
        /// `AllocatorCreateInfo::device`, and you want them to be used internally by this
        /// library:
        ///
        /// - VK_KHR_get_memory_requirements2
        /// - VK_KHR_dedicated_allocation
        ///
        /// When this flag is set, you can experience following warnings reported by Vulkan
        /// validation layer. You can ignore them.
        /// `> vkBindBufferMemory(): Binding memory to buffer 0x2d but vkGetBufferMemoryRequirements() has not been called on that buffer.`
        const KHR_DEDICATED_ALLOCATION = 0x0000_0002;

        /// Enables usage of VK_KHR_bind_memory2 extension.
        ///
        /// The flag works only if VmaAllocatorCreateInfo::vulkanApiVersion `== VK_API_VERSION_1_0`.
        /// When it is `VK_API_VERSION_1_1`, the flag is ignored because the extension has been promoted to Vulkan 1.1.
        ///
        /// You may set this flag only if you found out that this device extension is supported,
        /// you enabled it while creating Vulkan device passed as VmaAllocatorCreateInfo::device,
        /// and you want it to be used internally by this library.
        ///
        /// The extension provides functions `vkBindBufferMemory2KHR` and `vkBindImageMemory2KHR`,
        /// which allow to pass a chain of `pNext` structures while binding.
        /// This flag is required if you use `pNext` parameter in vmaBindBufferMemory2() or vmaBindImageMemory2().
        const VMA_ALLOCATOR_CREATE_KHR_BIND_MEMORY2_BIT = 0x00000004;

        /// Enables usage of VK_EXT_memory_budget extension.
        ///
        /// You may set this flag only if you found out that this device extension is supported,
        /// you enabled it while creating Vulkan device passed as VmaAllocatorCreateInfo::device,
        /// and you want it to be used internally by this library, along with another instance extension
        /// VK_KHR_get_physical_device_properties2, which is required by it (or Vulkan 1.1, where this extension is promoted).
        ///
        /// The extension provides query for current memory usage and budget, which will probably
        /// be more accurate than an estimation used by the library otherwise.
        const VMA_ALLOCATOR_CREATE_EXT_MEMORY_BUDGET_BIT = 0x00000008;

        /// Enables usage of VK_AMD_device_coherent_memory extension.
        ///
        /// You may set this flag only if you:
        ///
        /// - found out that this device extension is supported and enabled it while creating Vulkan device passed as VmaAllocatorCreateInfo::device,
        /// - checked that `VkPhysicalDeviceCoherentMemoryFeaturesAMD::deviceCoherentMemory` is true and set it while creating the Vulkan device,
        /// - want it to be used internally by this library.
        ///
        /// The extension and accompanying device feature provide access to memory types with
        /// `VK_MEMORY_PROPERTY_DEVICE_COHERENT_BIT_AMD` and `VK_MEMORY_PROPERTY_DEVICE_UNCACHED_BIT_AMD` flags.
        /// They are useful mostly for writing breadcrumb markers - a common method for debugging GPU crash/hang/TDR.
        ///
        /// When the extension is not enabled, such memory types are still enumerated, but their usage is illegal.
        /// To protect from this error, if you don't create the allocator with this flag, it will refuse to allocate any memory or create a custom pool in such memory type,
        /// returning `VK_ERROR_FEATURE_NOT_PRESENT`.
        const VMA_ALLOCATOR_CREATE_AMD_DEVICE_COHERENT_MEMORY_BIT = 0x00000010;

        /// Enables usage of "buffer device address" feature, which allows you to use function
        /// `vkGetBufferDeviceAddress*` to get raw GPU pointer to a buffer and pass it for usage inside a shader.
        ///
        /// You may set this flag only if you:
        ///
        /// 1. (For Vulkan version < 1.2) Found as available and enabled device extension
        /// VK_KHR_buffer_device_address.
        /// This extension is promoted to core Vulkan 1.2.
        /// 2. Found as available and enabled device feature `VkPhysicalDeviceBufferDeviceAddressFeatures::bufferDeviceAddress`.
        ///
        /// When this flag is set, you can create buffers with `VK_BUFFER_USAGE_SHADER_DEVICE_ADDRESS_BIT` using VMA.
        /// The library automatically adds `VK_MEMORY_ALLOCATE_DEVICE_ADDRESS_BIT` to
        /// allocated memory blocks wherever it might be needed.
        ///
        /// For more information, see documentation chapter \ref enabling_buffer_device_address.
        const VMA_ALLOCATOR_CREATE_BUFFER_DEVICE_ADDRESS_BIT = 0x00000020;

        /// Enables usage of VK_EXT_memory_priority extension in the library.
        ///
        /// You may set this flag only if you found available and enabled this device extension,
        /// along with `VkPhysicalDeviceMemoryPriorityFeaturesEXT::memoryPriority == VK_TRUE`,
        /// while creating Vulkan device passed as VmaAllocatorCreateInfo::device.
        ///
        /// When this flag is used, VmaAllocationCreateInfo::priority and VmaPoolCreateInfo::priority
        /// are used to set priorities of allocated Vulkan memory. Without it, these variables are ignored.
        ///
        /// A priority must be a floating-point value between 0 and 1, indicating the priority of the allocation relative to other memory allocations.
        /// Larger values are higher priority. The granularity of the priorities is implementation-dependent.
        /// It is automatically passed to every call to `vkAllocateMemory` done by the library using structure `VkMemoryPriorityAllocateInfoEXT`.
        /// The value to be used for default priority is 0.5.
        /// For more details, see the documentation of the VK_EXT_memory_priority extension.
        const VMA_ALLOCATOR_CREATE_EXT_MEMORY_PRIORITY_BIT = 0x00000040;

        const VMA_ALLOCATOR_CREATE_FLAG_BITS_MAX_ENUM = 0x7FFFFFFF;
    }
}

bitflags! {
    /// Flags for configuring `AllocatorPool` construction.
    pub struct AllocatorPoolCreateFlags: u32 {
        const NONE = 0x0000_0000;

        /// Use this flag if you always allocate only buffers and linear images or only optimal images
        /// out of this pool and so buffer-image granularity can be ignored.
        ///
        /// This is an optional optimization flag.
        ///
        /// If you always allocate using `Allocator::create_buffer`, `Allocator::create_image`,
        /// `Allocator::allocate_memory_for_buffer`, then you don't need to use it because allocator
        /// knows exact type of your allocations so it can handle buffer-image granularity
        /// in the optimal way.
        ///
        /// If you also allocate using `Allocator::allocate_memory_for_image` or `Allocator::allocate_memory`,
        /// exact type of such allocations is not known, so allocator must be conservative
        /// in handling buffer-image granularity, which can lead to suboptimal allocation
        /// (wasted memory). In that case, if you can make sure you always allocate only
        /// buffers and linear images or only optimal images out of this pool, use this flag
        /// to make allocator disregard buffer-image granularity and so make allocations
        /// faster and more optimal.
        const IGNORE_BUFFER_IMAGE_GRANULARITY = 0x0000_0002;

        /// Enables alternative, linear allocation algorithm in this pool.
        ///
        /// Specify this flag to enable linear allocation algorithm, which always creates
        /// new allocations after last one and doesn't reuse space from allocations freed in
        /// between. It trades memory consumption for simplified algorithm and data
        /// structure, which has better performance and uses less memory for metadata.
        ///
        /// By using this flag, you can achieve behavior of free-at-once, stack,
        /// ring buffer, and double stack.
        ///
        /// When using this flag, you must specify PoolCreateInfo::max_block_count == 1 (or 0 for default).
        const LINEAR_ALGORITHM = 0x0000_0004;

        /// Enables alternative, buddy allocation algorithm in this pool.
        ///
        /// It operates on a tree of blocks, each having size that is a power of two and
        /// a half of its parent's size. Comparing to default algorithm, this one provides
        /// faster allocation and deallocation and decreased external fragmentation,
        /// at the expense of more memory wasted (internal fragmentation).
        const BUDDY_ALGORITHM = 0x0000_0008;

        /// Bit mask to extract only `*_ALGORITHM` bits from entire set of flags.
        const ALGORITHM_MASK = 0x0000_0004 | 0x0000_0008;
    }
}

bitflags! {
    /// Flags for configuring `Allocation` construction.
    pub struct AllocationCreateFlags: u32 {
        /// Default configuration for allocation.
        const NONE = 0x0000_0000;

        /// Set this flag if the allocation should have its own memory block.
        ///
        /// Use it for special, big resources, like fullscreen images used as attachments.
        ///
        /// You should not use this flag if `AllocationCreateInfo::pool` is not `None`.
        const DEDICATED_MEMORY = 0x0000_0001;

        /// Set this flag to only try to allocate from existing `ash::vk::DeviceMemory` blocks and never create new such block.
        ///
        /// If new allocation cannot be placed in any of the existing blocks, allocation
        /// fails with `ash::vk::Result::ERROR_OUT_OF_DEVICE_MEMORY` error.
        ///
        /// You should not use `AllocationCreateFlags::DEDICATED_MEMORY` and `AllocationCreateFlags::NEVER_ALLOCATE` at the same time. It makes no sense.
        ///
        /// If `AllocationCreateInfo::pool` is not `None`, this flag is implied and ignored.
        const NEVER_ALLOCATE = 0x0000_0002;

        /// Set this flag to use a memory that will be persistently mapped and retrieve pointer to it.
        ///
        /// Pointer to mapped memory will be returned through `Allocation::get_mapped_data()`.
        ///
        /// Is it valid to use this flag for allocation made from memory type that is not
        /// `ash::vk::MemoryPropertyFlags::HOST_VISIBLE`. This flag is then ignored and memory is not mapped. This is
        /// useful if you need an allocation that is efficient to use on GPU
        /// (`ash::vk::MemoryPropertyFlags::DEVICE_LOCAL`) and still want to map it directly if possible on platforms that
        /// support it (e.g. Intel GPU).
        ///
        /// You should not use this flag together with `AllocationCreateFlags::CAN_BECOME_LOST`.
        const MAPPED = 0x0000_0004;

        /// Set this flag to treat `AllocationCreateInfo::user_data` as pointer to a
        /// null-terminated string. Instead of copying pointer value, a local copy of the
        /// string is made and stored in allocation's user data. The string is automatically
        /// freed together with the allocation. It is also used in `Allocator::build_stats_string`.
        const USER_DATA_COPY_STRING = 0x0000_0020;

        /// Allocation will be created from upper stack in a double stack pool.
        ///
        /// This flag is only allowed for custom pools created with `AllocatorPoolCreateFlags::LINEAR_ALGORITHM` flag.
        const UPPER_ADDRESS = 0x0000_0040;

        /// Create both buffer/image and allocation, but don't bind them together.
        /// It is useful when you want to bind yourself to do some more advanced binding, e.g. using some extensions.
        /// The flag is meaningful only with functions that bind by default, such as `Allocator::create_buffer`
        /// or `Allocator::create_image`. Otherwise it is ignored.
        const CREATE_DONT_BIND = 0x0000_0080;

        /// Create allocation only if additional device memory required for it, if any, won't exceed
        /// memory budget. Otherwise return `VK_ERROR_OUT_OF_DEVICE_MEMORY`.
        const WITHIN_BUDGET = 0x0000_0100;

        /// Set this flag if the allocated memory will have aliasing resources.
        ///
        /// Usage of this flag prevents supplying `VkMemoryDedicatedAllocateInfoKHR` when #const DEDICATED_MEMORY is specified.
        /// Otherwise created dedicated memory will not be suitable for aliasing resources, resulting in Vulkan Validation Layer errors.
        const CAN_ALIAS = 0x0000_0200;

        ///Requests possibility to map the allocation (using vmaMapMemory() or #const MAPPED_BIT).
        ///
        ///- If you use #VMA_MEMORY_USAGE_AUTO or other `VMA_MEMORY_USAGE_AUTO*` value,
        /// you must use this flag to be able to map the allocation. Otherwise, mapping is incorrect.
        ///- If you use other value of #VmaMemoryUsage, this flag is ignored and mapping is always possible in memory types that are `HOST_VISIBLE`.
        /// This includes allocations created in \ref custom_memory_pools.
        ///
        ///Declares that mapped memory will only be written sequentially, e.g. using `memcpy()` or a loop writing number-by-number,
        ///never read or accessed randomly, so a memory type can be selected that is uncached and write-combined.
        ///
        ///\warning Violating this declaration may work correctly, but will likely be very slow.
        ///Watch out for implicit reads introduced by doing e.g. `pMappedData[i] += x;`
        ///Better prepare your data in a local variable and `memcpy()` it to the mapped pointer all at once.
        const HOST_ACCESS_SEQUENTIAL_WRITE = 0x0000_0400;

        /// Requests possibility to map the allocation (using vmaMapMemory() or #const MAPPED_BIT).
        ///
        /// - If you use #VMA_MEMORY_USAGE_AUTO or other `VMA_MEMORY_USAGE_AUTO*` value,
        ///  you must use this flag to be able to map the allocation. Otherwise, mapping is incorrect.
        /// - If you use other value of #VmaMemoryUsage, this flag is ignored and mapping is always possible in memory types that are `HOST_VISIBLE`.
        ///  This includes allocations created in \ref custom_memory_pools.
        ///
        /// Declares that mapped memory can be read, written, and accessed in random order,
        /// so a `HOST_CACHED` memory type is required.
        const HOST_ACCESS_RANDOM = 0x0000_0800;

        /// Together with #const HOST_ACCESS_SEQUENTIAL_WRITE or #const HOST_ACCESS_RANDOM_BIT,
        /// it says that despite request for host access, a not-`HOST_VISIBLE` memory type can be selected
        /// if it may improve performance.

        /// By using this flag, you declare that you will check if the allocation ended up in a `HOST_VISIBLE` memory type
        /// (e.g. using vmaGetAllocationMemoryProperties()) and if not, you will create some "staging" buffer and
        /// issue an explicit transfer to write/read your data.
        /// To prepare for this possibility, don't forget to add appropriate flags like
        /// `VK_BUFFER_USAGE_TRANSFER_DST_BIT`, `VK_BUFFER_USAGE_TRANSFER_SRC_BIT` to the parameters of created buffer or image.
        const HOST_ACCESS_ALLOW_TRANSFER_INSTEAD = 0x0000_1000;

        /// Allocation strategy that chooses smallest possible free range for the allocation
        /// to minimize memory usage and fragmentation, possibly at the expense of allocation time.
        const STRATEGY_MIN_MEMORY = 0x0001_0000;

        /// Allocation strategy that chooses first suitable free range for the allocation -
        /// not necessarily in terms of the smallest offset but the one that is easiest and fastest to find
        /// to minimize allocation time, possibly at the expense of allocation quality.
        const STRATEGY_MIN_TIME = 0x0002_0000;

        /// Allocation strategy that chooses always the lowest offset in available space.
        /// This is not the most efficient strategy but achieves highly packed data.
        /// Used internally by defragmentation, not recomended in typical usage.
        const STRATEGY_MIN_OFFSET  = 0x0004_0000;

        /// Alias to #const STRATEGY_MIN_MEMORY_BIT.
        const STRATEGY_BEST_FIT = Self::STRATEGY_MIN_MEMORY.bits;

        /// Alias to #const STRATEGY_MIN_TIME_BIT.
        const STRATEGY_FIRST_FIT = Self::STRATEGY_MIN_TIME.bits;

        /// A bit mask to extract only `STRATEGY` bits from entire set of flags.
        const STRATEGY_MASK =
            Self::STRATEGY_MIN_MEMORY.bits |
            Self::STRATEGY_MIN_TIME.bits |
            Self::STRATEGY_MIN_OFFSET.bits;

        const FLAG_BITS_MAX_ENUM = 0x7FFF_FFFF;
    }
}

bitflags! {
    pub struct DefragmentationFlags: u32 {
        /// Use simple but fast algorithm for defragmentation.
        /// May not achieve best results but will require least time to compute and least allocations to copy.
        const ALGORITHM_FAST = 0x1;

        /// Default defragmentation algorithm, applied also when no `ALGORITHM` flag is specified.
        /// Offers a balance between defragmentation quality and the amount of allocations and bytes that need to be moved.
        const ALGORITHM_BALANCED = 0x2;

        /// Perform full defragmentation of memory.
        /// Can result in notably more time to compute and allocations to copy, but will achieve best memory packing.
        const ALGORITHM_FULL = 0x4;

        /// Use the most roboust algorithm at the cost of time to compute and number of copies to make.
        ///Only available when bufferImageGranularity is greater than 1, since it aims to reduce
        ///alignment issues between different types of resources.
        ///Otherwise falls back to same behavior as #VMA_DEFRAGMENTATION_FLAG_ALGORITHM_FULL_BIT.
        const ALGORITHM_EXTENSIVE = 0x8;

        /// A bit mask to extract only `ALGORITHM` bits from entire set of flags.
        const ALGORITHM_MASK =
            Self::ALGORITHM_FAST.bits |
            Self::ALGORITHM_BALANCED.bits |
            Self::ALGORITHM_FULL.bits |
            Self::ALGORITHM_EXTENSIVE.bits;

        const BITS_MAX_ENUM = 0x7FFFFFFF;
    }
}

bitflags! {
    pub struct VirtualAllocationCreateFlags: u32 {
        /// Allocation will be created from upper stack in a double stack pool.
        ///
        /// This flag is only allowed for virtual blocks created with #VMA_VIRTUAL_BLOCK_CREATE_LINEAR_ALGORITHM_BIT flag.
        const UPPER_ADDRESS = AllocationCreateFlags::UPPER_ADDRESS.bits;

        /// Allocation strategy that tries to minimize memory usage.
        const STRATEGY_MIN_MEMORY = AllocationCreateFlags::STRATEGY_MIN_MEMORY.bits;

        /// Allocation strategy that tries to minimize allocation time.
        const STRATEGY_MIN_TIME = AllocationCreateFlags::STRATEGY_MIN_TIME.bits;

        /// Allocation strategy that chooses always the lowest offset in available space.
        /// This is not the most efficient strategy but achieves highly packed data.
        const STRATEGY_MIN_OFFSET = AllocationCreateFlags::STRATEGY_MIN_OFFSET.bits;

        /// A bit mask to extract only `STRATEGY` bits from entire set of flags.
        ///
        /// These strategy flags are binary compatible with equivalent flags in #VmaAllocationCreateFlagBits.
        const STRATEGY_MASK = AllocationCreateFlags::STRATEGY_MASK.bits;

        const FLAG_BITS_MAX_ENUM = 0x7FFFFFFF;
    }
}

bitflags! {
    pub struct VirtualBlockCreateFlags: u32 {
        ///Enables alternative, linear allocation algorithm in this virtual block.
        ///
        ///Specify this flag to enable linear allocation algorithm, which always creates
        ///new allocations after last one and doesn't reuse space from allocations freed in
        ///between. It trades memory consumption for simplified algorithm and data
        ///structure, which has better performance and uses less memory for metadata.
        ///
        ///By using this flag, you can achieve behavior of free-at-once, stack,
        ///ring buffer, and double stack.
        ///For details, see documentation chapter \ref linear_algorithm.
        const LINEAR_ALGORITHM = 0x00000001;

        /// Bit mask to extract only `ALGORITHM` bits from entire set of flags.
        const ALGORITHM_MASK = Self::LINEAR_ALGORITHM.bits;

        const MAX_ENUM = 0x7FFFFFFF;
    }
}

/// Intended usage of memory.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub enum MemoryUsage {
    /// No intended memory usage specified.
    /// Use other members of VmaAllocationCreateInfo to specify your requirements.
    Unknown = 0,

    #[deprecated(
        since = "1.3.206",
        note = "Obsolete, preserved for backward compatibility. Prefers `ash::vk::MemoryPropertyFlags::DEVICE_LOCAL`"
    )]
    GpuOnly = 1,

    #[deprecated(
        since = "1.3.206",
        note = "Obsolete, preserved for backward compatibility. Guarantees `ash::vk::MemoryPropertyFlags::HOST_VISIBLE`, prefers `ash::vk::MemoryPropertyFlags::HOST_COHERENT`"
    )]
    CpuOnly = 2,

    #[deprecated(
        since = "1.3.206",
        note = "Obsolete, preserved for backward compatibility. Guarantees `ash::vk::MemoryPropertyFlags::HOST_VISIBLE`, prefers `ash::vk::MemoryPropertyFlags::DEVICE_LOCAL`"
    )]
    CpuToGpu = 3,

    #[deprecated(
        since = "1.3.206",
        note = "Obsolete, preserved for backward compatibility. Guarantees `ash::vk::MemoryPropertyFlags::HOST_VISIBLE`, prefers `ash::vk::MemoryPropertyFlags::HOST_CACHED`"
    )]
    GpuToCpu = 4,

    #[deprecated(
        since = "1.3.206",
        note = "Obsolete, preserved for backward compatibility. Prefers not `ash::vk::MemoryPropertyFlags::DEVICE_LOCAL`"
    )]
    CpuCopy = 5,

    /// Lazily allocated GPU memory having `VK_MEMORY_PROPERTY_LAZILY_ALLOCATED_BIT`.
    /// Exists mostly on mobile platforms. Using it on desktop PC or other GPUs with no such memory type present will fail the allocation.
    ///
    /// Usage: Memory for transient attachment images (color attachments, depth attachments etc.), created with `VK_IMAGE_USAGE_TRANSIENT_ATTACHMENT_BIT`
    ///
    /// Allocations with this usage are always created as dedicated - it implies #VMA_ALLOCATION_CREATE_DEDICATED_MEMORY_BIT.
    GpuLazilyAllocated = 6,

    /// Selects best memory type automatically.
    /// This flag is recommended for most common use cases.
    ///
    /// When using this flag, if you want to map the allocation (using vmaMapMemory() or #VMA_ALLOCATION_CREATE_MAPPED_BIT),
    /// you must pass one of the flags: #VMA_ALLOCATION_CREATE_HOST_ACCESS_SEQUENTIAL_WRITE_BIT or #VMA_ALLOCATION_CREATE_HOST_ACCESS_RANDOM_BIT
    /// in VmaAllocationCreateInfo::flags.
    ///
    /// It can be used only with functions that let the library know `VkBufferCreateInfo` or `VkImageCreateInfo`, e.g.
    /// vmaCreateBuffer(), vmaCreateImage(), vmaFindMemoryTypeIndexForBufferInfo(), vmaFindMemoryTypeIndexForImageInfo()
    /// and not with generic memory allocation functions.
    Auto = 7,

    /// Selects best memory type automatically with preference for GPU (device) memory.
    ///
    /// When using this flag, if you want to map the allocation (using vmaMapMemory() or #VMA_ALLOCATION_CREATE_MAPPED_BIT),
    /// you must pass one of the flags: #VMA_ALLOCATION_CREATE_HOST_ACCESS_SEQUENTIAL_WRITE_BIT or #VMA_ALLOCATION_CREATE_HOST_ACCESS_RANDOM_BIT
    /// in VmaAllocationCreateInfo::flags.
    ///
    /// It can be used only with functions that let the library know `VkBufferCreateInfo` or `VkImageCreateInfo`, e.g.
    /// vmaCreateBuffer(), vmaCreateImage(), vmaFindMemoryTypeIndexForBufferInfo(), vmaFindMemoryTypeIndexForImageInfo()
    /// and not with generic memory allocation functions.
    AutoPreferDevice = 8,

    /// Selects best memory type automatically with preference for CPU (host) memory.
    ///
    /// When using this flag, if you want to map the allocation (using vmaMapMemory() or #VMA_ALLOCATION_CREATE_MAPPED_BIT),
    /// you must pass one of the flags: #VMA_ALLOCATION_CREATE_HOST_ACCESS_SEQUENTIAL_WRITE_BIT or #VMA_ALLOCATION_CREATE_HOST_ACCESS_RANDOM_BIT
    /// in VmaAllocationCreateInfo::flags.
    ///
    /// It can be used only with functions that let the library know `VkBufferCreateInfo` or `VkImageCreateInfo`, e.g.
    /// vmaCreateBuffer(), vmaCreateImage(), vmaFindMemoryTypeIndexForBufferInfo(), vmaFindMemoryTypeIndexForImageInfo()
    /// and not with generic memory allocation functions.
    AutoPreferHost = 9,

    MaxEnum = 0x7FFFFFFF,
}

/// Operation performed on single defragmentation move. See structure #DefragmentationMove.
#[derive(Debug, Copy, Clone)]
pub enum DefragmentationMoveOperation {
    /// Buffer/image has been recreated at `dstTmpAllocation`, data has been copied, old buffer/image has been destroyed. `srcAllocation` should be changed to point to the new place. This is the default value set by vmaBeginDefragmentationPass().
    Copy = 0,

    /// Set this value if you cannot move the allocation. New place reserved at `dstTmpAllocation` will be freed. `srcAllocation` will remain unchanged.
    Ignore = 1,

    /// Set this value if you decide to abandon the allocation and you destroyed the buffer/image. New place reserved at `dstTmpAllocation` will be freed, along with `srcAllocation`, which will be destroyed.
    Destroy = 2,
}

/* #endregion Bitflags & Enums */

//

/* #region STRUCTURES */

/// Main allocator object
#[derive(Debug, Clone)]
pub struct Allocator {
    /// Pointer to internal VmaAllocator instance
    internal: ffi::VmaAllocator,
}

/// Represents custom memory pool handle.
///
/// Fill structure `AllocatorPoolCreateInfo` and call `Allocator::create_pool` to create it.
/// Call `Allocator::destroy_pool` to destroy it.
pub type AllocatorPool = ffi::VmaPool;

/// Represents single memory allocation.
///
/// It may be either dedicated block of `ash::vk::DeviceMemory` or a specific region of a
/// bigger block of this type plus unique offset.
///
/// Although the library provides convenience functions that create a Vulkan buffer or image,
/// allocate memory for it and bind them together, binding of the allocation to a buffer or an
/// image is out of scope of the allocation itself.
///
/// Allocation object can exist without buffer/image bound, binding can be done manually by
/// the user, and destruction of it can be done independently of destruction of the allocation.
///
/// The object also remembers its size and some other information. To retrieve this information,
/// use `Allocator::get_allocation_info`.
///
/// Some kinds allocations can be in lost state.
pub type Allocation = ffi::VmaAllocation;

pub struct DefragmentationContext {
    internal: ffi::VmaDefragmentationContext,
}

pub type VirtualAllocation = ffi::VmaVirtualAllocation;

pub struct VirtualBlock {
    internal: ffi::VmaVirtualBlock,
}

/// Callback function called after successful vkAllocateMemory.
pub type AllocateDeviceMemoryFunction = fn(
    allocator: Allocator,
    memoryType: u32,
    memory: vk::DeviceMemory,
    size: vk::DeviceSize,
    pUserData: *mut ::std::os::raw::c_void,
);

/// Callback function called before vkFreeMemory.
pub type FreeDeviceMemoryFunction = fn(
    allocator: Allocator,
    memoryType: u32,
    memory: vk::DeviceMemory,
    size: vk::DeviceSize,
    pUserData: *mut ::std::os::raw::c_void,
);

/// Set of callbacks that the library will call for `vkAllocateMemory` and `vkFreeMemory`.
///
/// Provided for informative purpose, e.g. to gather statistics about number of
/// allocations or total amount of memory allocated in Vulkan.
///
/// Used in VmaAllocatorCreateInfo::pDeviceMemoryCallbacks.
#[derive(Debug, Copy, Clone)]
pub struct DeviceMemoryCallbacks {
    /// Optional, can be null.
    pub pfn_allocate: Option<AllocateDeviceMemoryFunction>,

    /// Optional, can be null.
    pub pfn_free: Option<FreeDeviceMemoryFunction>,

    /// Optional, can be null.
    pub p_user_data: *mut ::std::os::raw::c_void,
}

// pub struct VmaVulkanFunctions ... // this structure is not needed for this wrapper

/// Description of an `Allocator` to be created.
pub struct AllocatorCreateInfo<'a> {
    /// Flags for created allocator. Use #AllocatorCreateFlags enum.
    pub flags: AllocatorCreateFlags,

    /// Vulkan physical device. It must be valid throughout whole lifetime of created allocator.
    pub physical_device: ash::vk::PhysicalDevice,

    /// Vulkan device. It must be valid throughout whole lifetime of created allocator.
    pub device: ash::Device,

    /// Preferred size of a single `ash::vk::DeviceMemory` block to be allocated from large heaps > 1 GiB.
    /// Set to 0 to use default, which is currently 256 MiB.
    pub preferred_large_heap_block_size: ash::vk::DeviceSize,

    /// Custom CPU memory allocation callbacks.
    pub allocation_callbacks: Option<vk::AllocationCallbacks>,

    /// Custom CPU memory allocation callbacks.
    pub device_memory_callbacks: Option<DeviceMemoryCallbacks>,

    /// Either empty or an array of limits on maximum number of bytes that can be allocated
    /// out of particular Vulkan memory heap.
    ///
    /// If not empty, it must contain `ash::vk::PhysicalDeviceMemoryProperties::memory_heap_count` elements,
    /// defining limit on maximum number of bytes that can be allocated out of particular Vulkan
    /// memory heap.
    ///
    /// Any of the elements may be equal to `ash::vk::WHOLE_SIZE`, which means no limit on that
    /// heap. This is also the default in case of an empty slice.
    ///
    /// If there is a limit defined for a heap:
    ///
    /// * If user tries to allocate more memory from that heap using this allocator, the allocation
    /// fails with `ash::vk::Result::ERROR_OUT_OF_DEVICE_MEMORY`.
    ///
    /// * If the limit is smaller than heap size reported in `ash::vk::MemoryHeap::size`, the value of this
    /// limit will be reported instead when using `Allocator::get_memory_properties`.
    ///
    /// Warning! Using this feature may not be equivalent to installing a GPU with smaller amount of
    /// memory, because graphics driver doesn't necessary fail new allocations with
    /// `ash::vk::Result::ERROR_OUT_OF_DEVICE_MEMORY` result when memory capacity is exceeded. It may return success
    /// and just silently migrate some device memory" blocks to system RAM. This driver behavior can
    /// also be controlled using the `VK_AMD_memory_overallocation_behavior` extension.
    pub heap_size_limit: Option<&'a [ash::vk::DeviceSize]>,

    // /// Pointers to Vulkan functions. Can be null.
    // /// For details see [Pointers to Vulkan functions](@ref config_Vulkan_functions)."]
    // pub pVulkanFunctions: *const VmaVulkanFunctions,
    /// Handle to Vulkan instance object.
    /// It must be valid throughout whole lifetime of created allocator.
    pub instance: ash::Instance,

    /// The highest version of Vulkan that the application is designed to use.
    /// It must be a value in the format as created by macro `VK_MAKE_VERSION` or a constant like:
    /// `VK_API_VERSION_1_1`, `VK_API_VERSION_1_0`. The patch version number specified is ignored.
    /// Only the major and minor versions are considered. It must be less or equal (preferably equal)
    /// to value as passed to `vkCreateInstance` as `VkApplicationInfo::apiVersion`. Only versions
    /// 1.0, 1.1, 1.2 are supported by the current implementation.
    /// Leaving it initialized to zero is equivalent to `VK_API_VERSION_1_0`.
    pub vulkan_api_version: u32,

    /// Either null or a pointer to an array of external memory handle types for each Vulkan memory type.
    ///
    /// If not NULL, it must be a pointer to an array of `VkPhysicalDeviceMemoryProperties::memoryTypeCount`
    /// elements, defining external memory handle types of particular Vulkan memory type,
    /// to be passed using `VkExportMemoryInfoKHR`.
    ///
    /// Any of the elements may be equal to 0, which means not to use `VkExportMemoryAllocateInfoKHR` on this memory type.
    /// This is also the default in case of `pTypeExternalMemoryHandleTypes` = NULL.
    pub external_memory_handle_type: *const vk::ExternalMemoryHandleTypeFlagsKHR,
}

/// Information about existing #Allocator object.
pub struct AllocatorInfo {
    /// Handle to Vulkan instance object.
    ///
    /// This is the same value as has been passed through AllocatorCreateInfo::instance.
    pub instance: vk::Instance,

    /// Handle to Vulkan physical device object.
    ///
    /// This is the same value as has been passed through AllocatorCreateInfo::physicalDevice.
    pub physical_device: vk::PhysicalDevice,

    /// Handle to Vulkan device object.
    ///
    /// This is the same value as has been passed through AllocatorCreateInfo::device.
    pub device: vk::Device,
}

/// Calculated statistics of memory usage e.g. in a specific memory type, heap, custom pool, or total.
///
/// These are fast to calculate.
/// See functions: vmaGetHeapBudgets(), vmaGetPoolStatistics().
#[derive(Clone, Copy)]
pub struct Statistics {
    /// Number of `VkDeviceMemory` objects - Vulkan memory blocks allocated.
    pub block_count: u32,

    /// Number of #Allocation objects allocated.
    ///
    /// Dedicated allocations have their own blocks, so each one adds 1 to `allocationCount` as well as `blockCount`.
    pub allocation_count: u32,

    /// Number of bytes allocated in `VkDeviceMemory` blocks.
    ///
    /// \\note To avoid confusion, please be aware that what Vulkan calls an \"allocation\" - a whole `VkDeviceMemory` object
    /// (e.g. as in `VkPhysicalDeviceLimits::maxMemoryAllocationCount`) is called a \"block\" in VMA, while VMA calls
    /// \"allocation\" a #Allocation object that represents a memory region sub-allocated from such block, usually for a single buffer or image.
    pub block_bytes: vk::DeviceSize,

    /// Total number of bytes occupied by all #Allocation objects.
    ///
    /// Always less or equal than `blockBytes`.
    /// Difference `(blockBytes - allocationBytes)` is the amount of memory allocated from Vulkan
    /// but unused by any #Allocation.
    pub allocation_bytes: vk::DeviceSize,
}

/// More detailed statistics than #Statistics.
///
/// These are slower to calculate. Use for debugging purposes.
/// See functions: vmaCalculateStatistics(), vmaCalculatePoolStatistics().
///
/// Previous version of the statistics API provided averages, but they have been removed
/// because they can be easily calculated as:
///
/// ```
/// VkDeviceSize allocationSizeAvg = detailedStats.statistics.allocationBytes / detailedStats.statistics.allocationCount;
/// VkDeviceSize unusedBytes = detailedStats.statistics.blockBytes - detailedStats.statistics.allocationBytes;
/// VkDeviceSize unusedRangeSizeAvg = unusedBytes / detailedStats.unusedRangeCount;
/// ```
pub struct DetailedStatistics {
    /// Basic statistics.
    pub statistics: Statistics,

    /// Number of free ranges of memory between allocations.
    pub unused_range_count: u32,

    /// Smallest allocation size. `VK_WHOLE_SIZE` if there are 0 allocations.
    pub allocation_size_min: vk::DeviceSize,

    /// Largest allocation size. 0 if there are 0 allocations.
    pub allocation_size_max: vk::DeviceSize,

    /// Smallest empty range size. `VK_WHOLE_SIZE` if there are 0 empty ranges.
    pub unused_range_size_min: vk::DeviceSize,

    /// Largest empty range size. 0 if there are 0 empty ranges.
    pub unused_range_size_max: vk::DeviceSize,
}

/// General statistics from current state of the Allocator -
/// total memory usage across all memory heaps and types.
///
/// These are slower to calculate. Use for debugging purposes.
/// See function vmaCalculateStatistics().
pub struct TotalStatistics {
    pub memory_type: [DetailedStatistics; 32usize],
    pub memory_heap: [DetailedStatistics; 16usize],
    pub total: DetailedStatistics,
}

/// Statistics of current memory usage and available budget for a specific memory heap.
///
/// These are fast to calculate.
/// See function vmaGetHeapBudgets().
#[derive(Clone, Copy)]
pub struct Budget {
    /// Statistics fetched from the library.
    pub statistics: Statistics,

    /// Estimated current memory usage of the program, in bytes.
    ///
    /// Fetched from system using VK_EXT_memory_budget extension if enabled.
    ///
    /// It might be different than `statistics.blockBytes` (usually higher) due to additional implicit objects
    /// also occupying the memory, like swapchain, pipelines, descriptor heaps, command buffers, or
    /// `VkDeviceMemory` blocks allocated outside of this library, if any.
    pub usage: vk::DeviceSize,

    /// Estimated amount of memory available to the program, in bytes.
    ///
    /// Fetched from system using VK_EXT_memory_budget extension if enabled.
    ///
    /// It might be different (most probably smaller) than `VkMemoryHeap::size[heapIndex]` due to factors
    /// external to the program, decided by the operating system.
    /// Difference `budget - usage` is the amount of additional memory that can probably
    /// be allocated without problems. Exceeding the budget may result in various problems.
    pub budget: vk::DeviceSize,
}

/// Parameters of new #Allocation.
///
/// To be used with functions like vmaCreateBuffer(), vmaCreateImage(), and many others.
pub struct AllocationCreateInfo {
    /// Use #AllocationCreateFlagBits enum.
    pub flags: AllocationCreateFlags,

    /// Intended usage of memory.
    ///
    /// You can leave #VMA_MEMORY_USAGE_UNKNOWN if you specify memory requirements in other way. \\n
    /// If `pool` is not null, this member is ignored.
    pub usage: MemoryUsage,

    /// Flags that must be set in a Memory Type chosen for an allocation.
    ///
    /// Leave 0 if you specify memory requirements in other way. \\n
    /// If `pool` is not null, this member is ignored.
    pub required_flags: vk::MemoryPropertyFlags,

    /// Flags that preferably should be set in a memory type chosen for an allocation.
    ///
    /// Set to 0 if no additional flags are preferred. \\n
    /// If `pool` is not null, this member is ignored.
    pub preferred_flags: vk::MemoryPropertyFlags,

    /// Bitmask containing one bit set for every memory type acceptable for this allocation.
    ///
    /// Value 0 is equivalent to `UINT32_MAX` - it means any memory type is accepted if
    /// it meets other requirements specified by this structure, with no further
    /// restrictions on memory type index. \\n
    /// If `pool` is not null, this member is ignored.
    pub memory_type_bits: u32,

    /// Pool that this allocation should be created in.
    ///
    /// Leave `VK_NULL_HANDLE` to allocate from default pool. If not null, members:
    /// `usage`, `requiredFlags`, `preferredFlags`, `memoryTypeBits` are ignored.
    pub pool: Option<AllocatorPool>,

    /// Custom general-purpose pointer that will be stored in #Allocation, can be read as AllocationInfo::pUserData and changed using vmaSetAllocationUserData().
    ///
    /// If #VMA_ALLOCATION_CREATE_USER_DATA_COPY_STRING_BIT is used, it must be either
    /// null or pointer to a null-terminated string. The string will be then copied to
    /// internal buffer, so it doesn't need to be valid after allocation call.
    pub p_user_data: *mut ::std::os::raw::c_void,

    /// A floating-point value between 0 and 1, indicating the priority of the allocation relative to other memory allocations.
    ///
    /// It is used only when #VMA_ALLOCATOR_CREATE_EXT_MEMORY_PRIORITY_BIT flag was used during creation of the #Allocator object
    /// and this allocation ends up as dedicated or is explicitly forced as dedicated using #VMA_ALLOCATION_CREATE_DEDICATED_MEMORY_BIT.
    /// Otherwise, it has the priority of a memory block where it is placed and this variable is ignored.
    pub priority: f32,
}

/// Description of an `AllocationPool` to be created.
#[derive(Debug, Clone)]
pub struct AllocatorPoolCreateInfo {
    /// Vulkan memory type index to allocate this pool from.
    pub memory_type_index: u32,

    /// Use combination of `AllocatorPoolCreateFlags`
    pub flags: AllocatorPoolCreateFlags,

    /// Size of a single `ash::vk::DeviceMemory` block to be allocated as part of this
    /// pool, in bytes.
    ///
    /// Specify non-zero to set explicit, constant size of memory blocks used by
    /// this pool.
    ///
    /// Leave 0 to use default and let the library manage block sizes automatically.
    /// Sizes of particular blocks may vary.
    pub block_size: vk::DeviceSize,

    /// Minimum number of blocks to be always allocated in this pool, even if they stay empty.
    ///
    /// Set to 0 to have no preallocated blocks and allow the pool be completely empty.
    pub min_block_count: usize,

    /// Maximum number of blocks that can be allocated in this pool.
    ///
    /// Set to 0 to use default, which is no limit.
    ///
    /// Set to same value as `AllocatorPoolCreateInfo::min_block_count` to have fixed amount
    /// of memory allocated throughout whole lifetime of this pool.
    pub max_block_count: usize,

    /// A floating-point value between 0 and 1, indicating the priority of the allocations in this pool relative to other memory /// ns.
    ///
    /// It is used only when #VMA_ALLOCATOR_CREATE_EXT_MEMORY_PRIORITY_BIT flag was used during creation of the #VmaAllocator object.
    /// Otherwise, this variable is ignored.
    pub priority: f32,

    /// Additional minimum alignment to be used for all allocations created from this pool. Can be 0.
    ///
    /// Leave 0 (default) not to impose any additional alignment. If not 0, it must be a power of two.
    /// It can be useful in cases where alignment returned by Vulkan by functions like `vkGetBufferMemoryRequirements` is not enough,
    /// e.g. when doing interop with OpenGL.
    pub min_allocation_alignment: vk::DeviceSize,

    /// Additional `pNext` chain to be attached to `VkMemoryAllocateInfo` used for every allocation made by this pool. Optional.
    ///
    /// Optional, can be null. If not null, it must point to a `pNext` chain of structures that can be attached to `VkMemoryAllocateInfo`.
    /// It can be useful for special needs such as adding `VkExportMemoryAllocateInfoKHR`.
    /// Structures pointed by this member must remain alive and unchanged for the whole lifetime of the custom pool.
    ///
    /// Please note that some structures, e.g. `VkMemoryPriorityAllocateInfoEXT`, `VkMemoryDedicatedAllocateInfoKHR`,
    /// can be attached automatically by this library when using other, more convenient of its features.
    pub p_memory_allocate_next: *mut ::std::os::raw::c_void,
}

/// Parameters of `Allocation` objects, that can be retrieved using `Allocator::get_allocation_info`.
#[derive(Debug, Clone)]
pub struct AllocationInfo {
    /// Pointer to internal VmaAllocationInfo instance
    internal: ffi::VmaAllocationInfo,
}

/// Parameters for defragmentation.
///
/// To be used with function BeginDefragmentation().
#[derive(Debug, Copy, Clone)]
pub struct DefragmentationInfo {
    // Use combination of #VmaDefragmentationFlagBits.
    pub flags: DefragmentationFlags,

    /// Custom pool to be defragmented.
    /// If null then default pools will undergo defragmentation process.
    pub pool: Option<AllocatorPool>,

    /// Maximum total numbers of bytes that can be copied while moving
    /// allocations to different places.
    ///
    /// Default is `ash::vk::WHOLE_SIZE`, which means no limit.
    pub max_bytes_per_pass: vk::DeviceSize,

    /// Maximum number of allocations that can be moved to different place.
    ///
    /// Default is `std::u32::MAX`, which means no limit.
    pub max_allocations_per_pass: u32,
}

/// Single move of an allocation to be done for defragmentation.
#[derive(Debug, Copy, Clone)]
pub struct DefragmentationMove {
    /// Operation to be performed on the allocation by vmaEndDefragmentationPass(). Default value is #VMA_DEFRAGMENTATION_MOVE_OPERATION_COPY. You can modify it."]
    pub operation: DefragmentationMoveOperation,

    /// Allocation that should be moved."]
    pub src_allocation: Allocation,

    /// Temporary allocation pointing to destination memory that will replace `srcAllocation`.
    ///
    /// __Do not store this allocation in your data structures! It exists only temporarily, for the duration of the defragmentation pass,
    /// to be used for binding new buffer/image to the destination memory using e.g. vmaBindBufferMemory().
    /// vmaEndDefragmentationPass() will destroy it and make `srcAllocation` point to this memory.___
    pub dst_tmp_allocation: Allocation,
}

/// Parameters for incremental defragmentation steps.
/// To be used with function BeginDefragmentationPass().
#[derive(Debug, Clone)]
pub struct DefragmentationPassMoveInfo {
    internal: ffi::VmaDefragmentationPassMoveInfo,
}

/// Statistics returned by `Allocator::defragment`
#[derive(Debug, Copy, Clone)]
pub struct DefragmentationStats {
    /// Total number of bytes that have been copied while moving allocations to different places.
    pub bytes_moved: vk::DeviceSize,

    /// Total number of bytes that have been released to the system by freeing empty `ash::vk::DeviceMemory` objects.
    pub bytes_freed: vk::DeviceSize,

    /// Number of allocations that have been moved to different places.
    pub allocations_moved: u32,

    /// Number of empty `ash::vk::DeviceMemory` objects that have been released to the system.
    pub device_memory_blocks_freed: u32,
}

/// Parameters of created #VmaVirtualBlock object to be passed to vmaCreateVirtualBlock().
pub struct VirtualBlockCreateInfo {
    /// Total size of the virtual block.
    ///
    /// Sizes can be expressed in bytes or any units you want as long as you are consistent in using them.
    /// For example, if you allocate from some array of structures, 1 can mean single instance of entire structure.
    pub size: vk::DeviceSize,

    /// Use combination of #VmaVirtualBlockCreateFlagBits.
    pub flags: VirtualBlockCreateFlags,

    /// Custom CPU memory allocation callbacks. Optional.
    ///
    /// Optional, can be null. When specified, they will be used for all CPU-side memory allocations.
    pub allocation_callbacks: Option<vk::AllocationCallbacks>,
}

/// Parameters of created virtual allocation to be passed to vmaVirtualAllocate().
pub struct VirtualAllocationCreateInfo {
    /// Size of the allocation.
    ///
    /// Cannot be zero.
    pub size: vk::DeviceSize,

    /// Required alignment of the allocation. Optional.
    ///
    /// Must be power of two. Special value 0 has the same meaning as 1 - means no special alignment is required, so allocation can start at any offset.
    pub alignment: Option<vk::DeviceSize>,

    /// Use combination of #VirtualAllocationCreateFlagBits.
    pub flags: VirtualAllocationCreateFlags,

    /// Custom pointer to be associated with the allocation. Optional.
    ///
    /// It can be any value and can be used for user-defined purposes. It can be fetched or changed later.
    pub p_user_data: *mut ::std::os::raw::c_void,
}

/// Parameters of an existing virtual allocation, returned by vmaGetVirtualAllocationInfo().
pub struct VirtualAllocationInfo {
    /// Offset of the allocation.
    ///
    /// Offset at which the allocation was made.
    pub offset: vk::DeviceSize,

    /// Size of the allocation.
    ///
    /// Same value as passed in VmaVirtualAllocationCreateInfo::size.
    pub size: vk::DeviceSize,

    /// Custom pointer associated with the allocation.
    ///
    /// Same value as passed in VmaVirtualAllocationCreateInfo::pUserData or to vmaSetVirtualAllocationUserData().
    pub p_user_data: *mut ::std::os::raw::c_void,
}

/* #endregion STRUCTURES */

/* #region FUNCTIONS & IMPLS */

// Allocator is internally thread safe unless AllocatorCreateFlags::EXTERNALLY_SYNCHRONIZED is used (then you need to add synchronization!)
unsafe impl Send for Allocator {}
unsafe impl Sync for Allocator {}

unsafe impl Send for AllocationInfo {}
unsafe impl Sync for AllocationInfo {}

unsafe impl Send for VirtualBlock {}
unsafe impl Sync for VirtualBlock {}

impl From<ffi::VmaStatistics> for Statistics {
    fn from(vma_statistics: ffi::VmaStatistics) -> Self {
        Statistics {
            block_count: vma_statistics.blockCount,
            allocation_count: vma_statistics.allocationCount,
            block_bytes: vma_statistics.blockBytes,
            allocation_bytes: vma_statistics.allocationBytes,
        }
    }
}

impl Into<ffi::VmaStatistics> for Statistics {
    fn into(self) -> ffi::VmaStatistics {
        ffi::VmaStatistics {
            blockCount: self.block_count,
            allocationCount: self.allocation_count,
            blockBytes: self.block_bytes,
            allocationBytes: self.allocation_bytes,
        }
    }
}

impl From<ffi::VmaDetailedStatistics> for DetailedStatistics {
    fn from(vma_statistics: ffi::VmaDetailedStatistics) -> Self {
        DetailedStatistics {
            statistics: vma_statistics.statistics.into(),
            unused_range_count: vma_statistics.unusedRangeCount,
            allocation_size_min: vma_statistics.allocationSizeMin,
            allocation_size_max: vma_statistics.allocationSizeMax,
            unused_range_size_min: vma_statistics.unusedRangeSizeMin,
            unused_range_size_max: vma_statistics.unusedRangeSizeMax,
        }
    }
}

impl Into<ffi::VmaDetailedStatistics> for DetailedStatistics {
    fn into(self) -> ffi::VmaDetailedStatistics {
        ffi::VmaDetailedStatistics {
            statistics: self.statistics.into(),
            unusedRangeCount: self.unused_range_count,
            allocationSizeMin: self.allocation_size_min,
            allocationSizeMax: self.allocation_size_max,
            unusedRangeSizeMin: self.unused_range_size_min,
            unusedRangeSizeMax: self.unused_range_size_max,
        }
    }
}

impl From<ffi::VmaTotalStatistics> for TotalStatistics {
    fn from(vma_statistics: ffi::VmaTotalStatistics) -> Self {
        TotalStatistics {
            memory_type: vma_statistics.memoryType.map(|value| value.into()),
            memory_heap: vma_statistics.memoryHeap.map(|value| value.into()),
            total: vma_statistics.total.into(),
        }
    }
}

impl Into<ffi::VmaTotalStatistics> for TotalStatistics {
    fn into(self) -> ffi::VmaTotalStatistics {
        ffi::VmaTotalStatistics {
            memoryType: self.memory_type.map(|value| value.into()),
            memoryHeap: self.memory_heap.map(|value| value.into()),
            total: self.total.into(),
        }
    }
}

impl AllocationInfo {
    #[inline(always)]
    // Gets the memory type index that this allocation was allocated from. (Never changes)
    pub fn get_memory_type(&self) -> u32 {
        self.internal.memoryType
    }

    /// Handle to Vulkan memory object.
    ///
    /// Same memory object can be shared by multiple allocations.
    ///
    /// It can change after call to `Allocator::defragment` if this allocation is passed
    /// to the function, or if allocation is lost.
    ///
    /// If the allocation is lost, it is equal to `ash::vk::DeviceMemory::null()`.
    #[inline(always)]
    pub fn get_device_memory(&self) -> ash::vk::DeviceMemory {
        self.internal.deviceMemory
    }

    /// Offset into device memory object to the beginning of this allocation, in bytes.
    /// (`self.get_device_memory()`, `self.get_offset()`) pair is unique to this allocation.
    ///
    /// It can change after call to `Allocator::defragment` if this allocation is passed
    /// to the function, or if allocation is lost.
    #[inline(always)]
    pub fn get_offset(&self) -> usize {
        self.internal.offset as usize
    }

    /// Size of this allocation, in bytes.
    ///
    /// It never changes, unless allocation is lost.
    #[inline(always)]
    pub fn get_size(&self) -> usize {
        self.internal.size as usize
    }

    /// Pointer to the beginning of this allocation as mapped data.
    ///
    /// If the allocation hasn't been mapped using `Allocator::map_memory` and hasn't been
    /// created with `AllocationCreateFlags::MAPPED` flag, this value is null.
    ///
    /// It can change after call to `Allocator::map_memory`, `Allocator::unmap_memory`.
    /// It can also change after call to `Allocator::defragment` if this allocation is
    /// passed to the function.
    #[inline(always)]
    pub fn get_mapped_data(&self) -> *mut u8 {
        self.internal.pMappedData as *mut u8
    }

    /*#[inline(always)]
    pub fn get_mapped_slice(&self) -> Option<&mut &[u8]> {
        if self.internal.pMappedData.is_null() {
            None
        } else {
            Some(unsafe { &mut ::std::slice::from_raw_parts(self.internal.pMappedData as *mut u8, self.get_size()) })
        }
    }*/

    /// Custom general-purpose pointer that was passed as `AllocationCreateInfo::user_data` or set using `Allocator::set_allocation_user_data`.
    ///
    /// It can change after a call to `Allocator::set_allocation_user_data` for this allocation.
    #[inline(always)]
    pub fn get_user_data(&self) -> *mut ::std::os::raw::c_void {
        self.internal.pUserData
    }
}

/// Converts a raw result into an ash result.
#[inline]
fn ffi_to_result(result: vk::Result) -> VkResult<()> {
    match result {
        vk::Result::SUCCESS => Ok(()),
        _ => Err(result),
    }
}

/// Converts an `AllocationCreateInfo` struct into the raw representation.
#[allow(deprecated)]
fn allocation_create_info_to_ffi(info: &AllocationCreateInfo) -> ffi::VmaAllocationCreateInfo {
    ffi::VmaAllocationCreateInfo {
        flags: info.flags.bits(),
        usage: match &info.usage {
            MemoryUsage::Unknown => ffi::VmaMemoryUsage_VMA_MEMORY_USAGE_UNKNOWN,
            MemoryUsage::GpuOnly => ffi::VmaMemoryUsage_VMA_MEMORY_USAGE_GPU_ONLY,
            MemoryUsage::CpuOnly => ffi::VmaMemoryUsage_VMA_MEMORY_USAGE_CPU_ONLY,
            MemoryUsage::CpuToGpu => ffi::VmaMemoryUsage_VMA_MEMORY_USAGE_CPU_TO_GPU,
            MemoryUsage::GpuToCpu => ffi::VmaMemoryUsage_VMA_MEMORY_USAGE_GPU_TO_CPU,
            MemoryUsage::CpuCopy => ffi::VmaMemoryUsage_VMA_MEMORY_USAGE_CPU_COPY,
            MemoryUsage::GpuLazilyAllocated => {
                ffi::VmaMemoryUsage_VMA_MEMORY_USAGE_GPU_LAZILY_ALLOCATED
            }
            MemoryUsage::Auto => ffi::VmaMemoryUsage_VMA_MEMORY_USAGE_AUTO,
            MemoryUsage::AutoPreferDevice => {
                ffi::VmaMemoryUsage_VMA_MEMORY_USAGE_AUTO_PREFER_DEVICE
            }
            MemoryUsage::AutoPreferHost => ffi::VmaMemoryUsage_VMA_MEMORY_USAGE_AUTO_PREFER_HOST,
            MemoryUsage::MaxEnum => ffi::VmaMemoryUsage_VMA_MEMORY_USAGE_MAX_ENUM,
        },
        requiredFlags: info.required_flags,
        preferredFlags: info.preferred_flags,
        memoryTypeBits: info.memory_type_bits,
        pool: match info.pool {
            Some(pool) => pool,
            None => ::std::ptr::null_mut(), // TODO // unsafe { mem::zeroed() },
        },
        pUserData: info.p_user_data,
        priority: 0.0,
    }
}

/// Converts an `AllocatorPoolCreateInfo` struct into the raw representation.
fn pool_create_info_to_ffi(info: &AllocatorPoolCreateInfo) -> ffi::VmaPoolCreateInfo {
    ffi::VmaPoolCreateInfo {
        memoryTypeIndex: info.memory_type_index,
        flags: info.flags.bits(),
        blockSize: info.block_size as vk::DeviceSize,
        minBlockCount: info.min_block_count,
        maxBlockCount: info.max_block_count,
        priority: 0.0,
        minAllocationAlignment: 0,
        pMemoryAllocateNext: ::std::ptr::null_mut(),
    }
}

impl Allocator {
    /// Constructor a new `Allocator` using the provided options.
    pub unsafe fn new(create_info: &AllocatorCreateInfo) -> VkResult<Self> {
        let instance = create_info.instance.clone();
        let device = create_info.device.clone();

        #[cfg(feature = "load_vulkan")]
        let entry = unsafe { ash::Entry::load().unwrap() };
        #[cfg(feature = "link_vulkan")]
        let entry = ash::Entry::linked();

        let routed_functions = ffi::VmaVulkanFunctions {
            vkGetPhysicalDeviceProperties: instance.fp_v1_0().get_physical_device_properties,
            vkGetPhysicalDeviceMemoryProperties: instance
                .fp_v1_0()
                .get_physical_device_memory_properties,
            vkAllocateMemory: device.fp_v1_0().allocate_memory,
            vkFreeMemory: device.fp_v1_0().free_memory,
            vkMapMemory: device.fp_v1_0().map_memory,
            vkUnmapMemory: device.fp_v1_0().unmap_memory,
            vkFlushMappedMemoryRanges: device.fp_v1_0().flush_mapped_memory_ranges,
            vkInvalidateMappedMemoryRanges: device.fp_v1_0().invalidate_mapped_memory_ranges,
            vkBindBufferMemory: device.fp_v1_0().bind_buffer_memory,
            vkBindImageMemory: device.fp_v1_0().bind_image_memory,
            vkGetBufferMemoryRequirements: device.fp_v1_0().get_buffer_memory_requirements,
            vkGetImageMemoryRequirements: device.fp_v1_0().get_image_memory_requirements,
            vkCreateBuffer: device.fp_v1_0().create_buffer,
            vkDestroyBuffer: device.fp_v1_0().destroy_buffer,
            vkCreateImage: device.fp_v1_0().create_image,
            vkDestroyImage: device.fp_v1_0().destroy_image,
            vkCmdCopyBuffer: device.fp_v1_0().cmd_copy_buffer,
            vkGetBufferMemoryRequirements2KHR: device.fp_v1_1().get_buffer_memory_requirements2,
            vkGetImageMemoryRequirements2KHR: device.fp_v1_1().get_image_memory_requirements2,
            vkBindBufferMemory2KHR: device.fp_v1_1().bind_buffer_memory2,
            vkBindImageMemory2KHR: device.fp_v1_1().bind_image_memory2,
            vkGetPhysicalDeviceMemoryProperties2KHR: instance
                .fp_v1_1()
                .get_physical_device_memory_properties2,
            vkGetInstanceProcAddr: entry.static_fn().get_instance_proc_addr,
            vkGetDeviceProcAddr: instance.fp_v1_0().get_device_proc_addr,
        };

        let allocation_callbacks = match create_info.allocation_callbacks {
            None => std::ptr::null(),
            Some(ref cb) => cb as *const _,
        };

        let ffi_create_info = ffi::VmaAllocatorCreateInfo {
            physicalDevice: create_info.physical_device,
            device: create_info.device.handle(),
            instance: instance.handle(),
            flags: create_info.flags.bits(),
            // frameInUseCount: create_info.frame_in_use_count,
            preferredLargeHeapBlockSize: create_info.preferred_large_heap_block_size as u64,
            pHeapSizeLimit: match &create_info.heap_size_limit {
                None => ::std::ptr::null(),
                Some(limits) => limits.as_ptr(),
            },
            pVulkanFunctions: &routed_functions,
            pAllocationCallbacks: allocation_callbacks,
            pDeviceMemoryCallbacks: ::std::ptr::null(), // TODO: Add support
            // pRecordSettings: ::std::ptr::null(),        // TODO: Add support
            vulkanApiVersion: create_info.vulkan_api_version,
            pTypeExternalMemoryHandleTypes: ::std::ptr::null(),
        };

        let mut internal: ffi::VmaAllocator = mem::zeroed();
        ffi_to_result(ffi::vmaCreateAllocator(
            &ffi_create_info as *const ffi::VmaAllocatorCreateInfo,
            &mut internal,
        ))?;

        Ok(Allocator { internal })
    }

    /// Destroys the internal allocator instance. After this has been called,
    /// no other functions may be called. Useful for ensuring a specific destruction
    /// order (for example, if an Allocator is a member of something that owns the Vulkan
    /// instance and destroys it in its own Drop).
    pub unsafe fn destroy(&mut self) {
        if !self.internal.is_null() {
            ffi::vmaDestroyAllocator(self.internal);
            self.internal = std::ptr::null_mut();
        }
    }

    /// Returns information about existing #VmaAllocator object - handle to Vulkan device etc.
    ///
    /// It might be useful if you want to keep just the #Allocator handle and fetch other required handles to
    /// `vk::PhysicalDevice`, `vk::Device` etc. every time using this function.
    pub unsafe fn get_info(&self) -> AllocatorInfo {
        let mut allocator_info: ffi::VmaAllocatorInfo = mem::zeroed();
        ffi::vmaGetAllocatorInfo(self.internal, &mut allocator_info);

        AllocatorInfo {
            instance: allocator_info.instance as vk::Instance,
            physical_device: allocator_info.physicalDevice as vk::PhysicalDevice,
            device: allocator_info.device as vk::Device,
        }
    }

    /// The allocator fetches `ash::vk::PhysicalDeviceProperties` from the physical device.
    /// You can get it here, without fetching it again on your own.
    pub unsafe fn get_physical_device_properties(&self) -> VkResult<vk::PhysicalDeviceProperties> {
        let mut properties = vk::PhysicalDeviceProperties::default();
        ffi::vmaGetPhysicalDeviceProperties(
            self.internal,
            &mut properties as *mut _ as *mut *const _,
        );

        Ok(properties)
    }

    /// The allocator fetches `ash::vk::PhysicalDeviceMemoryProperties` from the physical device.
    /// You can get it here, without fetching it again on your own.
    pub unsafe fn get_memory_properties(&self) -> VkResult<vk::PhysicalDeviceMemoryProperties> {
        let mut properties = vk::PhysicalDeviceMemoryProperties::default();
        ffi::vmaGetMemoryProperties(self.internal, &mut properties as *mut _ as *mut *const _);

        Ok(properties)
    }

    /// Given a memory type index, returns `ash::vk::MemoryPropertyFlags` of this memory type.
    ///
    /// This is just a convenience function; the same information can be obtained using
    /// `Allocator::get_memory_properties`.
    pub unsafe fn get_memory_type_properties(
        &self,
        memory_type_index: u32,
        flags: &mut vk::MemoryPropertyFlags,
    ) -> VkResult<vk::MemoryPropertyFlags> {
        ffi::vmaGetMemoryTypeProperties(self.internal, memory_type_index, flags);

        Ok(*flags)
    }

    /// Sets index of the current frame.
    ///
    /// This function must be used if you make allocations with `AllocationCreateFlags::CAN_BECOME_LOST` and
    /// `AllocationCreateFlags::CAN_MAKE_OTHER_LOST` flags to inform the allocator when a new frame begins.
    /// Allocations queried using `Allocator::get_allocation_info` cannot become lost
    /// in the current frame.
    pub unsafe fn set_current_frame_index(&self, frame_index: u32) {
        ffi::vmaSetCurrentFrameIndex(self.internal, frame_index);
    }

    /// Retrieves statistics from current state of the `Allocator`.
    pub unsafe fn calculate_statistics(
        &self,
        total_statistics: TotalStatistics,
    ) -> VkResult<ffi::VmaTotalStatistics> {
        let mut vma_stats: ffi::VmaTotalStatistics = total_statistics.into();
        ffi::vmaCalculateStatistics(self.internal, &mut vma_stats);
        Ok(vma_stats)
    }

    /// Retrieves information about current memory usage and budget for all memory heaps.
    ///
    /// allocator
    /// pBudgets Must point to array with number of elements at least equal to number of memory heaps in physical device used.
    ///
    /// This function is called \"get\" not \"calculate\" because it is very fast, suitable to be called
    /// every frame or every allocation. For more detailed statistics use vmaCalculateStatistics().
    ///
    /// Note that when using allocator from multiple threads, returned information may immediately
    /// become outdated.
    pub fn get_heap_budgets(&self, budget_count: usize) -> Vec<Budget> {
        unsafe {
            let mut budgets = Vec::<ffi::VmaBudget>::with_capacity(budget_count);
            budgets.resize_with(budget_count, || mem::zeroed());
            ffi::vmaGetHeapBudgets(self.internal, budgets.as_mut_ptr());
            budgets
                .iter()
                .map(|value| Budget {
                    statistics: Statistics {
                        block_count: value.statistics.blockCount,
                        allocation_count: value.statistics.allocationCount,
                        block_bytes: value.statistics.blockBytes,
                        allocation_bytes: value.statistics.allocationBytes,
                    },
                    usage: value.usage,
                    budget: value.budget,
                })
                .collect::<Vec<Budget>>()
        }
    }

    /// Helps to find memory type index, given memory type bits and allocation info.
    ///
    /// This algorithm tries to find a memory type that:
    ///
    /// - Is allowed by memory type bits.
    /// - Contains all the flags from `allocation_info.required_flags`.
    /// - Matches intended usage.
    /// - Has as many flags from `allocation_info.preferred_flags` as possible.
    ///
    /// Returns ash::vk::Result::ERROR_FEATURE_NOT_PRESENT if not found. Receiving such a result
    /// from this function or any other allocating function probably means that your
    /// device doesn't support any memory type with requested features for the specific
    /// type of resource you want to use it for. Please check parameters of your
    /// resource, like image layout (OPTIMAL versus LINEAR) or mip level count.
    pub unsafe fn find_memory_type_index(
        &self,
        memory_type_bits: u32,
        allocation_info: &AllocationCreateInfo,
    ) -> VkResult<u32> {
        let create_info = allocation_create_info_to_ffi(&allocation_info);
        let mut memory_type_index: u32 = 0;
        ffi_to_result(ffi::vmaFindMemoryTypeIndex(
            self.internal,
            memory_type_bits,
            &create_info,
            &mut memory_type_index,
        ))?;

        Ok(memory_type_index)
    }

    /// Helps to find memory type index, given buffer info and allocation info.
    ///
    /// It can be useful e.g. to determine value to be used as `AllocatorPoolCreateInfo::memory_type_index`.
    /// It internally creates a temporary, dummy buffer that never has memory bound.
    /// It is just a convenience function, equivalent to calling:
    ///
    /// - `ash::vk::Device::create_buffer`
    /// - `ash::vk::Device::get_buffer_memory_requirements`
    /// - `Allocator::find_memory_type_index`
    /// - `ash::vk::Device::destroy_buffer`
    pub unsafe fn find_memory_type_index_for_buffer_info(
        &self,
        buffer_info: ash::vk::BufferCreateInfo,
        allocation_info: &AllocationCreateInfo,
    ) -> VkResult<u32> {
        let allocation_create_info = allocation_create_info_to_ffi(&allocation_info);
        let mut memory_type_index: u32 = 0;
        ffi_to_result(ffi::vmaFindMemoryTypeIndexForBufferInfo(
            self.internal,
            &buffer_info,
            &allocation_create_info,
            &mut memory_type_index,
        ))?;

        Ok(memory_type_index)
    }

    /// Helps to find memory type index, given image info and allocation info.
    ///
    /// It can be useful e.g. to determine value to be used as `AllocatorPoolCreateInfo::memory_type_index`.
    /// It internally creates a temporary, dummy image that never has memory bound.
    /// It is just a convenience function, equivalent to calling:
    ///
    /// - `ash::vk::Device::create_image`
    /// - `ash::vk::Device::get_image_memory_requirements`
    /// - `Allocator::find_memory_type_index`
    /// - `ash::vk::Device::destroy_image`
    pub unsafe fn find_memory_type_index_for_image_info(
        &self,
        image_info: ash::vk::ImageCreateInfo,
        allocation_info: &AllocationCreateInfo,
    ) -> VkResult<u32> {
        let allocation_create_info = allocation_create_info_to_ffi(&allocation_info);
        let mut memory_type_index: u32 = 0;
        ffi_to_result(ffi::vmaFindMemoryTypeIndexForImageInfo(
            self.internal,
            &image_info,
            &allocation_create_info,
            &mut memory_type_index,
        ))?;

        Ok(memory_type_index)
    }

    /// Allocates Vulkan device memory and creates `AllocatorPool` object.
    pub unsafe fn create_pool(
        &self,
        pool_info: &AllocatorPoolCreateInfo,
    ) -> VkResult<AllocatorPool> {
        let mut ffi_pool: ffi::VmaPool = mem::zeroed();
        let create_info = pool_create_info_to_ffi(&pool_info);
        ffi_to_result(ffi::vmaCreatePool(
            self.internal,
            &create_info,
            &mut ffi_pool,
        ))?;
        Ok(ffi_pool)
    }

    /// Destroys `AllocatorPool` object and frees Vulkan device memory.
    pub unsafe fn destroy_pool(&self, pool: AllocatorPool) {
        ffi::vmaDestroyPool(self.internal, pool);
    }

    /// Retrieves statistics of existing `AllocatorPool` object.
    pub unsafe fn get_pool_statistics(
        &self,
        pool: AllocatorPool,
    ) -> VkResult<ffi::VmaDetailedStatistics> {
        let mut pool_stats: ffi::VmaDetailedStatistics = mem::zeroed();
        ffi::vmaCalculatePoolStatistics(self.internal, pool, &mut pool_stats);
        Ok(pool_stats)
    }

    /// Retrieves detailed statistics of existing #VmaPool object.
    ///
    /// allocator Allocator object.
    /// pool Pool object.
    /// pPoolStats Statistics of specified pool.
    pub fn calculate_pool_statistics(&self, pool: &AllocatorPool) -> DetailedStatistics {
        unsafe {
            let mut vma_detailed_stats: ffi::VmaDetailedStatistics = mem::zeroed();
            ffi::vmaCalculatePoolStatistics(self.internal, *pool, &mut vma_detailed_stats);
            vma_detailed_stats.into()
        }
    }

    /// Checks magic number in margins around all allocations in given memory pool in search for corruptions.
    ///
    /// Corruption detection is enabled only when `VMA_DEBUG_DETECT_CORRUPTION` macro is defined to nonzero,
    /// `VMA_DEBUG_MARGIN` is defined to nonzero and the pool is created in memory type that is
    /// `ash::vk::MemoryPropertyFlags::HOST_VISIBLE` and `ash::vk::MemoryPropertyFlags::HOST_COHERENT`.
    ///
    /// Possible error values:
    ///
    /// - `ash::vk::Result::ERROR_FEATURE_NOT_PRESENT` - corruption detection is not enabled for specified pool.
    /// - `ash::vk::Result::ERROR_VALIDATION_FAILED_EXT` - corruption detection has been performed and found memory corruptions around one of the allocations.
    ///  `VMA_ASSERT` is also fired in that case.
    /// - Other value: Error returned by Vulkan, e.g. memory mapping failure.
    pub unsafe fn check_pool_corruption(&self, pool: AllocatorPool) -> VkResult<()> {
        ffi_to_result(ffi::vmaCheckPoolCorruption(self.internal, pool))
    }

    /// Retrieves name of a custom pool.
    ///
    /// After the call `ppName` is either null or points to an internally-owned null-terminated string
    /// containing name of the pool that was previously set. The pointer becomes invalid when the pool is
    /// destroyed or its name is changed using vmaSetPoolName().
    pub fn get_pool_name(&self, pool: &AllocatorPool) -> &str {
        unsafe {
            let c_name: *mut *const ::std::os::raw::c_char = mem::zeroed();
            ffi::vmaGetPoolName(self.internal, *pool, c_name);
            std::ffi::CStr::from_ptr(*c_name).to_str().unwrap()
        }
    }

    /// Sets name of a custom pool.
    ///
    /// `pName` can be either null or pointer to a null-terminated string with new name for the pool.
    /// Function makes internal copy of the string, so it can be changed or freed immediately after this call.
    pub fn set_pool_name(&self, pool: &AllocatorPool, name: String) {
        unsafe {
            let c_name = std::ffi::CString::new(name).unwrap();
            ffi::vmaSetPoolName(self.internal, *pool, c_name.as_ptr())
        };
    }

    /// General purpose memory allocation.
    ///
    /// You should free the memory using `Allocator::free_memory` or 'Allocator::free_memory_pages'.
    ///
    /// It is recommended to use `Allocator::allocate_memory_for_buffer`, `Allocator::allocate_memory_for_image`,
    /// `Allocator::create_buffer`, `Allocator::create_image` instead whenever possible.
    pub unsafe fn allocate_memory(
        &self,
        memory_requirements: &ash::vk::MemoryRequirements,
        allocation_info: &AllocationCreateInfo,
    ) -> VkResult<(Allocation, AllocationInfo)> {
        let create_info = allocation_create_info_to_ffi(&allocation_info);
        let mut allocation: Allocation = mem::zeroed();
        let mut allocation_info: AllocationInfo = mem::zeroed();
        ffi_to_result(ffi::vmaAllocateMemory(
            self.internal,
            memory_requirements,
            &create_info,
            &mut allocation,
            &mut allocation_info.internal,
        ))?;

        Ok((allocation, allocation_info))
    }

    /// General purpose memory allocation for multiple allocation objects at once.
    ///
    /// You should free the memory using `Allocator::free_memory` or `Allocator::free_memory_pages`.
    ///
    /// Word "pages" is just a suggestion to use this function to allocate pieces of memory needed for sparse binding.
    /// It is just a general purpose allocation function able to make multiple allocations at once.
    /// It may be internally optimized to be more efficient than calling `Allocator::allocate_memory` `allocations.len()` times.
    ///
    /// All allocations are made using same parameters. All of them are created out of the same memory pool and type.
    pub unsafe fn allocate_memory_pages(
        &self,
        memory_requirements: &ash::vk::MemoryRequirements,
        allocation_info: &AllocationCreateInfo,
        allocation_count: usize,
    ) -> VkResult<Vec<(Allocation, AllocationInfo)>> {
        let create_info = allocation_create_info_to_ffi(&allocation_info);
        let mut allocations: Vec<ffi::VmaAllocation> = vec![mem::zeroed(); allocation_count];
        let mut allocation_info: Vec<ffi::VmaAllocationInfo> =
            vec![mem::zeroed(); allocation_count];
        ffi_to_result(ffi::vmaAllocateMemoryPages(
            self.internal,
            memory_requirements,
            &create_info,
            allocation_count,
            allocations.as_mut_ptr(),
            allocation_info.as_mut_ptr(),
        ))?;

        let it = allocations.iter().zip(allocation_info.iter());
        let allocations: Vec<(Allocation, AllocationInfo)> = it
            .map(|(alloc, info)| (*alloc, AllocationInfo { internal: *info }))
            .collect();

        Ok(allocations)
    }

    /// Buffer specialized memory allocation.
    ///
    /// You should free the memory using `Allocator::free_memory` or 'Allocator::free_memory_pages'.
    pub unsafe fn allocate_memory_for_buffer(
        &self,
        buffer: ash::vk::Buffer,
        allocation_info: &AllocationCreateInfo,
    ) -> VkResult<(Allocation, AllocationInfo)> {
        let create_info = allocation_create_info_to_ffi(&allocation_info);
        let mut allocation: Allocation = mem::zeroed();
        let mut allocation_info: AllocationInfo = mem::zeroed();
        ffi_to_result(ffi::vmaAllocateMemoryForBuffer(
            self.internal,
            buffer,
            &create_info,
            &mut allocation,
            &mut allocation_info.internal,
        ))?;

        Ok((allocation, allocation_info))
    }

    /// Image specialized memory allocation.
    ///
    /// You should free the memory using `Allocator::free_memory` or 'Allocator::free_memory_pages'.
    pub unsafe fn allocate_memory_for_image(
        &self,
        image: ash::vk::Image,
        allocation_info: &AllocationCreateInfo,
    ) -> VkResult<(Allocation, AllocationInfo)> {
        let create_info = allocation_create_info_to_ffi(&allocation_info);
        let mut allocation: Allocation = mem::zeroed();
        let mut allocation_info: AllocationInfo = mem::zeroed();
        ffi_to_result(ffi::vmaAllocateMemoryForImage(
            self.internal,
            image,
            &create_info,
            &mut allocation,
            &mut allocation_info.internal,
        ))?;

        Ok((allocation, allocation_info))
    }

    /// Frees memory previously allocated using `Allocator::allocate_memory`,
    /// `Allocator::allocate_memory_for_buffer`, or `Allocator::allocate_memory_for_image`.
    pub unsafe fn free_memory(&self, allocation: &Allocation) {
        ffi::vmaFreeMemory(self.internal, *allocation);
    }

    /// Frees memory and destroys multiple allocations.
    ///
    /// Word "pages" is just a suggestion to use this function to free pieces of memory used for sparse binding.
    /// It is just a general purpose function to free memory and destroy allocations made using e.g. `Allocator::allocate_memory',
    /// 'Allocator::allocate_memory_pages` and other functions.
    ///
    /// It may be internally optimized to be more efficient than calling 'Allocator::free_memory` `allocations.len()` times.
    ///
    /// Allocations in 'allocations' slice can come from any memory pools and types.
    pub unsafe fn free_memory_pages(&self, allocations: &[Allocation]) {
        ffi::vmaFreeMemoryPages(
            self.internal,
            allocations.len(),
            allocations.as_ptr() as *mut _,
        );
    }

    /// Returns current information about specified allocation and atomically marks it as used in current frame.
    ///
    /// Current parameters of given allocation are returned in the result object, available through accessors.
    ///
    /// This function also atomically "touches" allocation - marks it as used in current frame,
    /// just like `Allocator::touch_allocation`.
    ///
    /// If the allocation is in lost state, `allocation.get_device_memory` returns `ash::vk::DeviceMemory::null()`.
    ///
    /// Although this function uses atomics and doesn't lock any mutex, so it should be quite efficient,
    /// you can avoid calling it too often.
    ///
    /// If you just want to check if allocation is not lost, `Allocator::touch_allocation` will work faster.
    pub unsafe fn get_allocation_info(&self, allocation: &Allocation) -> VkResult<AllocationInfo> {
        let mut allocation_info: AllocationInfo = mem::zeroed();
        ffi::vmaGetAllocationInfo(self.internal, *allocation, &mut allocation_info.internal);
        Ok(allocation_info)
    }

    /// Sets user data in given allocation to new value.
    ///
    /// If the allocation was created with `AllocationCreateFlags::USER_DATA_COPY_STRING`,
    /// `user_data` must be either null, or pointer to a null-terminated string. The function
    /// makes local copy of the string and sets it as allocation's user data. String
    /// passed as user data doesn't need to be valid for whole lifetime of the allocation -
    /// you can free it after this call. String previously pointed by allocation's
    /// user data is freed from memory.
    ///
    /// If the flag was not used, the value of pointer `user_data` is just copied to
    /// allocation's user data. It is opaque, so you can use it however you want - e.g.
    /// as a pointer, ordinal number or some handle to you own data.
    pub unsafe fn set_allocation_user_data(
        &self,
        allocation: &Allocation,
        p_user_data: *mut ::std::os::raw::c_void,
    ) {
        ffi::vmaSetAllocationUserData(self.internal, *allocation, p_user_data);
    }

    /// Sets pName in given allocation to new value.
    ///
    /// `pName` must be either null, or pointer to a null-terminated string. The function
    /// makes local copy of the string and sets it as allocation's `pName`. String
    /// passed as pName doesn't need to be valid for whole lifetime of the allocation -
    /// you can free it after this call. String previously pointed by allocation's
    /// `pName` is freed from memory.
    pub fn set_allocation_name(&self, allocation: &Allocation, name: String) {
        let c_name = std::ffi::CString::new(name).unwrap();
        unsafe {
            ffi::vmaSetAllocationName(self.internal, *allocation, c_name.as_ptr());
        };
    }

    /// Given an allocation, returns Property Flags of its memory type.
    ///
    /// This is just a convenience function. Same information can be obtained using
    /// vmaGetAllocationInfo() + vmaGetMemoryProperties().
    pub fn get_allocation_memory_properties(
        &self,
        allocation: &Allocation,
    ) -> vk::MemoryPropertyFlags {
        let mut p_flags: vk::MemoryPropertyFlags = unsafe { mem::zeroed() };
        unsafe { ffi::vmaGetAllocationMemoryProperties(self.internal, *allocation, &mut p_flags) };
        p_flags
    }

    /// Maps memory represented by given allocation and returns pointer to it.
    ///
    /// Maps memory represented by given allocation to make it accessible to CPU code.
    /// When succeeded, result is a pointer to first byte of this memory.
    ///
    /// If the allocation is part of bigger `ash::vk::DeviceMemory` block, the pointer is
    /// correctly offseted to the beginning of region assigned to this particular
    /// allocation.
    ///
    /// Mapping is internally reference-counted and synchronized, so despite raw Vulkan
    /// function `ash::vk::Device::MapMemory` cannot be used to map same block of
    /// `ash::vk::DeviceMemory` multiple times simultaneously, it is safe to call this
    /// function on allocations assigned to the same memory block. Actual Vulkan memory
    /// will be mapped on first mapping and unmapped on last unmapping.
    ///
    /// If the function succeeded, you must call `Allocator::unmap_memory` to unmap the
    /// allocation when mapping is no longer needed or before freeing the allocation, at
    /// the latest.
    ///
    /// It also safe to call this function multiple times on the same allocation. You
    /// must call `Allocator::unmap_memory` same number of times as you called
    /// `Allocator::map_memory`.
    ///
    /// It is also safe to call this function on allocation created with
    /// `AllocationCreateFlags::MAPPED` flag. Its memory stays mapped all the time.
    /// You must still call `Allocator::unmap_memory` same number of times as you called
    /// `Allocator::map_memory`. You must not call `Allocator::unmap_memory` additional
    /// time to free the "0-th" mapping made automatically due to `AllocationCreateFlags::MAPPED` flag.
    ///
    /// This function fails when used on allocation made in memory type that is not
    /// `ash::vk::MemoryPropertyFlags::HOST_VISIBLE`.
    ///
    /// This function always fails when called for allocation that was created with
    /// `AllocationCreateFlags::CAN_BECOME_LOST` flag. Such allocations cannot be mapped.
    pub unsafe fn map_memory(&self, allocation: &Allocation) -> VkResult<*mut u8> {
        let mut mapped_data: *mut ::std::os::raw::c_void = ::std::ptr::null_mut();
        ffi_to_result(ffi::vmaMapMemory(
            self.internal,
            *allocation,
            &mut mapped_data,
        ))?;

        Ok(mapped_data as *mut u8)
    }

    /// Unmaps memory represented by given allocation, mapped previously using `Allocator::map_memory`.
    pub unsafe fn unmap_memory(&self, allocation: &Allocation) {
        ffi::vmaUnmapMemory(self.internal, *allocation);
    }

    /// Flushes memory of given allocation.
    ///
    /// Calls `ash::vk::Device::FlushMappedMemoryRanges` for memory associated with given range of given allocation.
    ///
    /// - `offset` must be relative to the beginning of allocation.
    /// - `size` can be `ash::vk::WHOLE_SIZE`. It means all memory from `offset` the the end of given allocation.
    /// - `offset` and `size` don't have to be aligned; hey are internally rounded down/up to multiple of `nonCoherentAtomSize`.
    /// - If `size` is 0, this call is ignored.
    /// - If memory type that the `allocation` belongs to is not `ash::vk::MemoryPropertyFlags::HOST_VISIBLE` or it is `ash::vk::MemoryPropertyFlags::HOST_COHERENT`, this call is ignored.
    pub unsafe fn flush_allocation(
        &self,
        allocation: &Allocation,
        offset: usize,
        size: usize,
    ) -> VkResult<()> {
        ffi_to_result(ffi::vmaFlushAllocation(
            self.internal,
            *allocation,
            offset as vk::DeviceSize,
            size as vk::DeviceSize,
        ))
    }

    /// Invalidates memory of given allocation.
    ///
    /// Calls `ash::vk::Device::invalidate_mapped_memory_ranges` for memory associated with given range of given allocation.
    ///
    /// - `offset` must be relative to the beginning of allocation.
    /// - `size` can be `ash::vk::WHOLE_SIZE`. It means all memory from `offset` the the end of given allocation.
    /// - `offset` and `size` don't have to be aligned. They are internally rounded down/up to multiple of `nonCoherentAtomSize`.
    /// - If `size` is 0, this call is ignored.
    /// - If memory type that the `allocation` belongs to is not `ash::vk::MemoryPropertyFlags::HOST_VISIBLE` or it is `ash::vk::MemoryPropertyFlags::HOST_COHERENT`, this call is ignored.
    pub unsafe fn invalidate_allocation(
        &self,
        allocation: &Allocation,
        offset: usize,
        size: usize,
    ) -> VkResult<()> {
        ffi_to_result(ffi::vmaInvalidateAllocation(
            self.internal,
            *allocation,
            offset as vk::DeviceSize,
            size as vk::DeviceSize,
        ))
    }

    /// Flushes memory of given set of allocations.
    ///
    /// Calls `vkFlushMappedMemoryRanges()` for memory associated with given ranges of given allocations.
    /// For more information, see documentation of vmaFlushAllocation().
    ///
    /// allocator
    /// allocationCount
    /// allocations
    /// offsets If not null, it must point to an array of offsets of regions to flush, relative to the beginning of respective allocations. Null means all ofsets are zero.
    /// sizes If not null, it must point to an array of sizes of regions to flush in respective allocations. Null means `VK_WHOLE_SIZE` for all allocations.
    ///
    /// This function returns the `VkResult` from `vkFlushMappedMemoryRanges` if it is
    /// called, otherwise `VK_SUCCESS`.
    pub fn flush_allocations(
        &self,
        allocations: &mut [Allocation],
        offsets: &[vk::DeviceSize],
        sizes: &[vk::DeviceSize],
    ) -> VkResult<()> {
        unsafe {
            ffi_to_result(ffi::vmaFlushAllocations(
                self.internal,
                allocations.len() as u32,
                allocations.as_mut_ptr(),
                offsets.as_ptr(),
                sizes.as_ptr(),
            ))
        }
    }

    /// Invalidates memory of given set of allocations.
    ///
    /// Calls `vkInvalidateMappedMemoryRanges()` for memory associated with given ranges of given allocations.
    /// For more information, see documentation of vmaInvalidateAllocation().
    ///
    /// allocator
    /// allocationCount
    /// allocations
    /// offsets If not null, it must point to an array of offsets of regions to flush, relative to the beginning of respective allocations. Null means all ofsets are zero.
    /// sizes If not null, it must point to an array of sizes of regions to flush in respective allocations. Null means `VK_WHOLE_SIZE` for all allocations.
    ///
    /// This function returns the `VkResult` from `vkInvalidateMappedMemoryRanges` if it is
    /// called, otherwise `VK_SUCCESS`.
    pub fn invalidate_allocations(
        &self,
        allocations: &mut [Allocation],
        offsets: &[vk::DeviceSize],
        sizes: &[vk::DeviceSize],
    ) -> VkResult<()> {
        unsafe {
            ffi_to_result(ffi::vmaInvalidateAllocations(
                self.internal,
                allocations.len() as u32,
                allocations.as_mut_ptr(),
                offsets.as_ptr(),
                sizes.as_ptr(),
            ))
        }
    }

    /// Checks magic number in margins around all allocations in given memory types (in both default and custom pools) in search for corruptions.
    ///
    /// `memory_type_bits` bit mask, where each bit set means that a memory type with that index should be checked.
    ///
    /// Corruption detection is enabled only when `VMA_DEBUG_DETECT_CORRUPTION` macro is defined to nonzero,
    /// `VMA_DEBUG_MARGIN` is defined to nonzero and only for memory types that are `HOST_VISIBLE` and `HOST_COHERENT`.
    ///
    /// Possible error values:
    ///
    /// - `ash::vk::Result::ERROR_FEATURE_NOT_PRESENT` - corruption detection is not enabled for any of specified memory types.
    /// - `ash::vk::Result::ERROR_VALIDATION_FAILED_EXT` - corruption detection has been performed and found memory corruptions around one of the allocations.
    ///  `VMA_ASSERT` is also fired in that case.
    /// - Other value: Error returned by Vulkan, e.g. memory mapping failure.
    pub unsafe fn check_corruption(
        &self,
        memory_types: ash::vk::MemoryPropertyFlags,
    ) -> VkResult<()> {
        ffi_to_result(ffi::vmaCheckCorruption(
            self.internal,
            memory_types.as_raw(),
        ))
    }

    /// Begins defragmentation process.
    ///
    /// Use this function instead of old, deprecated `Allocator::defragment`.
    ///
    /// Warning! Between the call to `Allocator::defragmentation_begin` and `Allocator::defragmentation_end`.
    ///
    /// - You should not use any of allocations passed as `allocations` or
    /// any allocations that belong to pools passed as `pools`,
    /// including calling `Allocator::get_allocation_info`, `Allocator::touch_allocation`, or access
    /// their data.
    ///
    /// - Some mutexes protecting internal data structures may be locked, so trying to
    /// make or free any allocations, bind buffers or images, map memory, or launch
    /// another simultaneous defragmentation in between may cause stall (when done on
    /// another thread) or deadlock (when done on the same thread), unless you are
    /// 100% sure that defragmented allocations are in different pools.
    ///
    /// - Information returned via stats and `info.allocations_changed` are undefined.
    /// They become valid after call to `Allocator::defragmentation_end`.
    ///
    /// - If `info.command_buffer` is not null, you must submit that command buffer
    /// and make sure it finished execution before calling `Allocator::defragmentation_end`.
    pub unsafe fn begin_defragmentation(
        &self,
        info: &DefragmentationInfo,
    ) -> VkResult<DefragmentationContext> {
        let mut context = DefragmentationContext {
            internal: mem::zeroed(),
        };

        let ffi_info = ffi::VmaDefragmentationInfo {
            flags: info.flags.bits, // Reserved for future use'
            pool: info.pool.unwrap_or(std::ptr::null_mut()),
            maxBytesPerPass: info.max_bytes_per_pass,
            maxAllocationsPerPass: info.max_allocations_per_pass,
        };

        ffi_to_result(ffi::vmaBeginDefragmentation(
            self.internal,
            &ffi_info,
            &mut context.internal,
        ))?;

        Ok(context)
    }

    /// Ends defragmentation process.
    ///
    /// Use this function to finish defragmentation started by `Allocator::defragmentation_begin`.
    pub unsafe fn end_defragmentation(
        &self,
        context: &mut DefragmentationContext,
    ) -> VkResult<DefragmentationStats> {
        let mut vma_defrag_stats: ffi::VmaDefragmentationStats = mem::zeroed();
        ffi::vmaEndDefragmentation(self.internal, context.internal, &mut vma_defrag_stats);

        let stats = DefragmentationStats {
            bytes_moved: vma_defrag_stats.bytesMoved,
            bytes_freed: vma_defrag_stats.bytesFreed,
            allocations_moved: vma_defrag_stats.allocationsMoved,
            device_memory_blocks_freed: vma_defrag_stats.deviceMemoryBlocksFreed,
        };

        Ok(stats)
    }

    /// Starts single defragmentation pass.
    ///
    /// allocator Allocator object.
    /// context Context object that has been created by vmaBeginDefragmentation().
    /// pPassInfo Computed informations for current pass.
    ///
    /// - `VK_SUCCESS` if no more moves are possible. Then you can omit call to vmaEndDefragmentationPass() and simply end whole defragmentation.
    /// - `VK_INCOMPLETE` if there are pending moves returned in `pPassInfo`. You need to perform them, call vmaEndDefragmentationPass(),
    /// and then preferably try another pass with vmaBeginDefragmentationPass().
    pub fn begin_defragmentation_pass(
        &self,
        context: &mut DefragmentationContext,
    ) -> (VkResult<()>, DefragmentationPassMoveInfo) {
        let mut pass_info: ffi::VmaDefragmentationPassMoveInfo = unsafe { mem::zeroed() };
        unsafe {
            let result = ffi_to_result(ffi::vmaBeginDefragmentationPass(
                self.internal,
                context.internal,
                &mut pass_info,
            ));

            (
                result,
                DefragmentationPassMoveInfo {
                    internal: pass_info,
                },
            )
        }
    }

    /// Ends single defragmentation pass.
    ///
    /// allocator Allocator object.
    /// context Context object that has been created by vmaBeginDefragmentation().
    /// pPassInfo Computed informations for current pass filled by vmaBeginDefragmentationPass() and possibly modified by you.
    ///
    /// Returns `VK_SUCCESS` if no more moves are possible or `VK_INCOMPLETE` if more defragmentations are possible.
    ///
    /// Ends incremental defragmentation pass and commits all defragmentation moves from `pPassInfo`.
    /// After this call:
    ///
    /// - Allocations at `pPassInfo[i].srcAllocation` that had `pPassInfo[i].operation ==` #VMA_DEFRAGMENTATION_MOVE_OPERATION_COPY
    /// (which is the default) will be pointing to the new destination place.
    /// - Allocation at `pPassInfo[i].srcAllocation` that had `pPassInfo[i].operation ==` #VMA_DEFRAGMENTATION_MOVE_OPERATION_DESTROY
    /// will be freed.
    ///
    /// If no more moves are possible you can end whole defragmentation.
    pub fn end_defragmentation_pass(
        &self,
        context: &mut DefragmentationContext,
        move_pass_info: &mut DefragmentationPassMoveInfo,
    ) -> VkResult<()> {
        unsafe {
            ffi_to_result(ffi::vmaEndDefragmentationPass(
                self.internal,
                context.internal,
                &mut move_pass_info.internal,
            ))
        }
    }

    /// Binds buffer to allocation.
    ///
    /// Binds specified buffer to region of memory represented by specified allocation.
    /// Gets `ash::vk::DeviceMemory` handle and offset from the allocation.
    ///
    /// If you want to create a buffer, allocate memory for it and bind them together separately,
    /// you should use this function for binding instead of `ash::vk::Device::bind_buffer_memory`,
    /// because it ensures proper synchronization so that when a `ash::vk::DeviceMemory` object is
    /// used by multiple allocations, calls to `ash::vk::Device::bind_buffer_memory()` or
    /// `ash::vk::Device::map_memory()` won't happen from multiple threads simultaneously
    /// (which is illegal in Vulkan).
    ///
    /// It is recommended to use function `Allocator::create_buffer` instead of this one.
    pub unsafe fn bind_buffer_memory(
        &self,
        buffer: ash::vk::Buffer,
        allocation: &Allocation,
    ) -> VkResult<()> {
        ffi_to_result(ffi::vmaBindBufferMemory(self.internal, *allocation, buffer))
    }

    /// Binds buffer to allocation with additional parameters.
    ///
    /// allocator
    /// allocation
    /// allocationLocalOffset Additional offset to be added while binding, relative to the beginning of the `allocation`. Normally it should be 0.
    /// buffer
    /// pNext A chain of structures to be attached to `VkBindBufferMemoryInfoKHR` structure used internally. Normally it should be null.
    ///
    /// This function is similar to vmaBindBufferMemory(), but it provides additional parameters.
    ///
    /// If `pNext` is not null, #VmaAllocator object must have been created with #VMA_ALLOCATOR_CREATE_KHR_BIND_MEMORY2_BIT flag
    /// or with VmaAllocatorCreateInfo::vulkanApiVersion `>= VK_API_VERSION_1_1`. Otherwise the call fails.

    pub unsafe fn bind_buffer_memory2<T>(
        &self,
        buffer: ash::vk::Buffer,
        allocation: &Allocation,
        allocation_local_offset: vk::DeviceSize,
        p_next: T,
    ) -> VkResult<()>
    where
        T: Into<Option<*mut ::std::os::raw::c_void>>,
    {
        ffi_to_result(ffi::vmaBindBufferMemory2(
            self.internal,
            *allocation,
            allocation_local_offset,
            buffer,
            if let Some(p_next_val) = p_next.into() {
                p_next_val
            } else {
                std::ptr::null_mut()
            },
        ))
    }

    /// Binds image to allocation.
    ///
    /// Binds specified image to region of memory represented by specified allocation.
    /// Gets `ash::vk::DeviceMemory` handle and offset from the allocation.
    ///
    /// If you want to create a image, allocate memory for it and bind them together separately,
    /// you should use this function for binding instead of `ash::vk::Device::bind_image_memory`,
    /// because it ensures proper synchronization so that when a `ash::vk::DeviceMemory` object is
    /// used by multiple allocations, calls to `ash::vk::Device::bind_image_memory()` or
    /// `ash::vk::Device::map_memory()` won't happen from multiple threads simultaneously
    /// (which is illegal in Vulkan).
    ///
    /// It is recommended to use function `Allocator::create_image` instead of this one.
    pub unsafe fn bind_image_memory(
        &self,
        image: ash::vk::Image,
        allocation: &Allocation,
    ) -> VkResult<()> {
        ffi_to_result(ffi::vmaBindImageMemory(self.internal, *allocation, image))
    }

    /// Binds image to allocation with additional parameters.
    ///
    /// allocator
    /// allocation
    /// allocationLocalOffset Additional offset to be added while binding, relative to the beginning of the `allocation`. Normally it should be 0.
    /// image
    /// pNext A chain of structures to be attached to `VkBindImageMemoryInfoKHR` structure used internally. Normally it should be null.
    ///
    /// This function is similar to vmaBindImageMemory(), but it provides additional parameters.
    ///
    /// If `pNext` is not null, #VmaAllocator object must have been created with #VMA_ALLOCATOR_CREATE_KHR_BIND_MEMORY2_BIT flag
    /// or with VmaAllocatorCreateInfo::vulkanApiVersion `>= VK_API_VERSION_1_1`. Otherwise the call fails.
    pub unsafe fn bind_image_memory2<T>(
        &self,
        image: ash::vk::Image,
        allocation: &Allocation,
        allocation_local_offset: vk::DeviceSize,
        p_next: T,
    ) -> VkResult<()>
    where
        T: Into<Option<*mut ::std::os::raw::c_void>>,
    {
        ffi_to_result(ffi::vmaBindImageMemory2(
            self.internal,
            *allocation,
            allocation_local_offset,
            image,
            if let Some(p_next_val) = p_next.into() {
                p_next_val
            } else {
                std::ptr::null_mut()
            },
        ))
    }

    /// Creates a new `VkBuffer`, allocates and binds memory for it.
    ///
    /// allocator
    /// pBufferCreateInfo
    /// pAllocationCreateInfo
    /// pBuffer Buffer that was created.
    /// pAllocation Allocation that was created.
    /// pAllocationInfo Optional. Information about allocated memory. It can be later fetched using function vmaGetAllocationInfo().
    ///
    /// This function automatically:
    ///
    /// -# Creates buffer.
    /// -# Allocates appropriate memory for it.
    /// -# Binds the buffer with the memory.
    ///
    /// If any of these operations fail, buffer and allocation are not created,
    /// returned value is negative error code, `*pBuffer` and `*pAllocation` are null.
    ///
    /// If the function succeeded, you must destroy both buffer and allocation when you
    /// no longer need them using either convenience function vmaDestroyBuffer() or
    /// separately, using `vkDestroyBuffer()` and vmaFreeMemory().
    ///
    /// If #VMA_ALLOCATOR_CREATE_KHR_DEDICATED_ALLOCATION_BIT flag was used,
    /// VK_KHR_dedicated_allocation extension is used internally to query driver whether
    /// it requires or prefers the new buffer to have dedicated allocation. If yes,
    /// and if dedicated allocation is possible
    /// (#VMA_ALLOCATION_CREATE_NEVER_ALLOCATE_BIT is not used), it creates dedicated
    /// allocation for this buffer, just like when using
    /// #VMA_ALLOCATION_CREATE_DEDICATED_MEMORY_BIT.
    ///
    /// This function creates a new `VkBuffer`. Sub-allocation of parts of one large buffer,
    /// although recommended as a good practice, is out of scope of this library and could be implemented
    /// by the user as a higher-level logic on top of VMA.
    ///
    /// This function automatically creates a buffer, allocates appropriate memory
    /// for it, and binds the buffer with the memory.
    ///
    /// If the function succeeded, you must destroy both buffer and allocation when you
    /// no longer need them using either convenience function `Allocator::destroy_buffer` or
    /// separately, using `ash::Device::destroy_buffer` and `Allocator::free_memory`.
    ///
    /// If `AllocatorCreateFlags::KHR_DEDICATED_ALLOCATION` flag was used,
    /// VK_KHR_dedicated_allocation extension is used internally to query driver whether
    /// it requires or prefers the new buffer to have dedicated allocation. If yes,
    /// and if dedicated allocation is possible (AllocationCreateInfo::pool is null
    /// and `AllocationCreateFlags::NEVER_ALLOCATE` is not used), it creates dedicated
    /// allocation for this buffer, just like when using `AllocationCreateFlags::DEDICATED_MEMORY`.
    pub unsafe fn create_buffer(
        &self,
        buffer_info: &ash::vk::BufferCreateInfo,
        allocation_info: &AllocationCreateInfo,
    ) -> VkResult<(ash::vk::Buffer, Allocation, AllocationInfo)> {
        let allocation_create_info = allocation_create_info_to_ffi(&allocation_info);
        let mut buffer = vk::Buffer::null();
        let mut allocation: Allocation = mem::zeroed();
        let mut allocation_info: AllocationInfo = mem::zeroed();
        ffi_to_result(ffi::vmaCreateBuffer(
            self.internal,
            &*buffer_info,
            &allocation_create_info,
            &mut buffer,
            &mut allocation,
            &mut allocation_info.internal,
        ))?;

        Ok((buffer, allocation, allocation_info))
    }

    /// Creates a buffer with additional minimum alignment.
    ///
    /// Similar to vmaCreateBuffer() but provides additional parameter `minAlignment` which allows to specify custom,
    /// minimum alignment to be used when placing the buffer inside a larger memory block, which may be needed e.g.
    /// for interop with OpenGL.
    pub fn create_buffer_with_alignment(
        &self,
        buffer_info: &ash::vk::BufferCreateInfo,
        allocation_info: &AllocationCreateInfo,
        min_alignment: vk::DeviceSize,
    ) -> VkResult<(ash::vk::Buffer, Allocation, AllocationInfo)> {
        let allocation_create_info = allocation_create_info_to_ffi(&allocation_info);
        let mut buffer = vk::Buffer::null();
        unsafe {
            let mut allocation: Allocation = mem::zeroed();
            let mut allocation_info: AllocationInfo = mem::zeroed();
            ffi_to_result(ffi::vmaCreateBufferWithAlignment(
                self.internal,
                &*buffer_info,
                &allocation_create_info,
                min_alignment,
                &mut buffer,
                &mut allocation,
                &mut allocation_info.internal,
            ))?;

            Ok((buffer, allocation, allocation_info))
        }
    }

    /// Creates a new `VkBuffer`, binds already created memory for it.
    ///
    /// allocator
    /// allocation Allocation that provides memory to be used for binding new buffer to it.
    /// pBufferCreateInfo
    /// pBuffer Buffer that was created.
    ///
    /// This function automatically:
    ///
    /// -# Creates buffer.
    /// -# Binds the buffer with the supplied memory.
    ///
    /// If any of these operations fail, buffer is not created,
    /// returned value is negative error code and `*pBuffer` is null.
    ///
    /// If the function succeeded, you must destroy the buffer when you
    /// no longer need it using `vkDestroyBuffer()`. If you want to also destroy the corresponding
    /// allocation you can use convenience function vmaDestroyBuffer().
    pub fn create_aliasing_buffer(
        &self,
        allocation: &Allocation,
        buffer_info: &ash::vk::BufferCreateInfo,
    ) -> VkResult<vk::Buffer> {
        let mut buffer = vk::Buffer::null();
        unsafe {
            ffi_to_result(ffi::vmaCreateAliasingBuffer(
                self.internal,
                *allocation,
                &*buffer_info,
                &mut buffer,
            ))?
        };

        Ok(buffer)
    }

    /// Destroys Vulkan buffer and frees allocated memory.
    ///
    /// This is just a convenience function equivalent to:
    ///
    /// ```ignore
    /// ash::vk::Device::destroy_buffer(buffer, None);
    /// Allocator::free_memory(allocator, allocation);
    /// ```
    ///
    /// It it safe to pass null as `buffer` and/or `allocation`.
    pub unsafe fn destroy_buffer(&self, buffer: ash::vk::Buffer, allocation: &Allocation) {
        ffi::vmaDestroyBuffer(self.internal, buffer, *allocation);
    }

    /// This function automatically creates an image, allocates appropriate memory
    /// for it, and binds the image with the memory.
    ///
    /// If the function succeeded, you must destroy both image and allocation when you
    /// no longer need them using either convenience function `Allocator::destroy_image` or
    /// separately, using `ash::Device::destroy_image` and `Allocator::free_memory`.
    ///
    /// If `AllocatorCreateFlags::KHR_DEDICATED_ALLOCATION` flag was used,
    /// `VK_KHR_dedicated_allocation extension` is used internally to query driver whether
    /// it requires or prefers the new image to have dedicated allocation. If yes,
    /// and if dedicated allocation is possible (AllocationCreateInfo::pool is null
    /// and `AllocationCreateFlags::NEVER_ALLOCATE` is not used), it creates dedicated
    /// allocation for this image, just like when using `AllocationCreateFlags::DEDICATED_MEMORY`.
    ///
    /// If `VK_ERROR_VALIDAITON_FAILED_EXT` is returned, VMA may have encountered a problem
    /// that is not caught by the validation layers. One example is if you try to create a 0x0
    /// image, a panic will occur and `VK_ERROR_VALIDAITON_FAILED_EXT` is thrown.
    pub unsafe fn create_image(
        &self,
        image_info: &ash::vk::ImageCreateInfo,
        allocation_info: &AllocationCreateInfo,
    ) -> VkResult<(ash::vk::Image, Allocation, AllocationInfo)> {
        let allocation_create_info = allocation_create_info_to_ffi(&allocation_info);
        let mut image = vk::Image::null();
        let mut allocation: Allocation = mem::zeroed();
        let mut allocation_info: AllocationInfo = mem::zeroed();
        ffi_to_result(ffi::vmaCreateImage(
            self.internal,
            &*image_info,
            &allocation_create_info,
            &mut image,
            &mut allocation,
            &mut allocation_info.internal,
        ))?;

        Ok((image, allocation, allocation_info))
    }

    /// Function similar to vmaCreateAliasingBuffer().
    pub fn create_aliasing_image(
        &self,
        allocation: &Allocation,
        image_info: &ash::vk::ImageCreateInfo,
    ) -> VkResult<vk::Image> {
        let mut image = vk::Image::null();
        unsafe {
            ffi_to_result(ffi::vmaCreateAliasingImage(
                self.internal,
                *allocation,
                &*image_info,
                &mut image,
            ))?
        };

        Ok(image)
    }

    /// Destroys Vulkan image and frees allocated memory.
    ///
    /// This is just a convenience function equivalent to:
    ///
    /// ```ignore
    /// ash::vk::Device::destroy_image(image, None);
    /// Allocator::free_memory(allocator, allocation);
    /// ```
    ///
    /// It it safe to pass null as `image` and/or `allocation`.
    pub fn destroy_image(&self, image: ash::vk::Image, allocation: &Allocation) {
        unsafe { ffi::vmaDestroyImage(self.internal, image, *allocation) };
    }

    /// Builds and returns statistics as a String in JSON format.
    /// detailed_map
    pub fn build_stats_string(&self, detailed_map: bool) -> VkResult<String> {
        let mut stats_string: *mut ::std::os::raw::c_char = ::std::ptr::null_mut();
        unsafe {
            ffi::vmaBuildStatsString(
                self.internal,
                &mut stats_string,
                if detailed_map { 1 } else { 0 },
            );
        }

        Ok(if stats_string.is_null() {
            String::new()
        } else {
            unsafe {
                let result = std::ffi::CStr::from_ptr(stats_string)
                    .to_string_lossy()
                    .into_owned();
                ffi::vmaFreeStatsString(self.internal, stats_string);
                result
            }
        })
    }
}

impl VirtualBlock {
    /// Creates new #VmaVirtualBlock object.
    ///
    /// pCreateInfo Parameters for creation.
    /// pVirtualBlock Returned virtual block object or `VMA_NULL` if creation failed.
    pub fn new(create_info: VirtualBlockCreateInfo) -> VkResult<Self> {
        let allocation_callbacks = match create_info.allocation_callbacks {
            None => std::ptr::null(),
            Some(ref cb) => cb as *const _,
        };

        let ffi_create_info = ffi::VmaVirtualBlockCreateInfo {
            size: create_info.size,
            flags: create_info.flags.bits,
            pAllocationCallbacks: allocation_callbacks,
        };

        let mut internal: ffi::VmaVirtualBlock = unsafe { mem::zeroed() };
        unsafe {
            ffi_to_result(ffi::vmaCreateVirtualBlock(
                &ffi_create_info as *const ffi::VmaVirtualBlockCreateInfo,
                &mut internal,
            ))?
        };

        Ok(Self { internal })
    }

    /// Destroys #VmaVirtualBlock object.
    ///
    /// Please note that you should consciously handle virtual allocations that could remain unfreed in the block.
    /// You should either free them individually using vmaVirtualFree() or call vmaClearVirtualBlock()
    /// if you are sure this is what you want. If you do neither, an assert is called.
    ///
    /// If you keep pointers to some additional metadata associated with your virtual allocations in their `pUserData`,
    /// don't forget to free them.
    pub fn destroy(self) {
        unsafe { ffi::vmaDestroyVirtualBlock(self.internal) }
    }

    /// Returns true of the #VmaVirtualBlock is empty - contains 0 virtual allocations and has all its space available for new allocations.
    pub fn is_empty(&self) -> bool {
        unsafe {
            if ffi::vmaIsVirtualBlockEmpty(self.internal) == 0 {
                false
            } else {
                true
            }
        }
    }

    /// Returns information about a specific virtual allocation within a virtual block, like its size and `pUserData` pointer.
    pub fn get_virtual_allocation_info(
        &self,
        allocation: VirtualAllocation,
    ) -> VirtualAllocationInfo {
        let mut vma_vallocation_info: ffi::VmaVirtualAllocationInfo = unsafe { mem::zeroed() };
        unsafe {
            ffi::vmaGetVirtualAllocationInfo(self.internal, allocation, &mut vma_vallocation_info)
        };

        VirtualAllocationInfo {
            offset: vma_vallocation_info.offset,
            size: vma_vallocation_info.size,
            p_user_data: vma_vallocation_info.pUserData,
        }
    }

    /// Allocates new virtual allocation inside given #VmaVirtualBlock.
    ///
    /// If the allocation fails due to not enough free space available, `VK_ERROR_OUT_OF_DEVICE_MEMORY` is returned
    /// (despite the function doesn't ever allocate actual GPU memory).
    /// `pAllocation` is then set to `VK_NULL_HANDLE` and `pOffset`, if not null, it set to `UINT64_MAX`.
    ///
    /// virtualBlock Virtual block
    /// pCreateInfo Parameters for the allocation
    /// pAllocation Returned handle of the new allocation
    /// pOffset Returned offset of the new allocation. Optional, can be null.
    pub fn allocate<T1, T2, T3>(
        &mut self,
        size: vk::DeviceSize,
        alignment: T1,
        flags: T2,
        p_user_data: T3,
    ) -> VkResult<VirtualAllocation>
    where
        T1: Into<Option<vk::DeviceSize>>,
        T2: Into<Option<VirtualAllocationCreateFlags>>,
        T3: Into<Option<*mut ::std::os::raw::c_void>>,
    {
        let valloc_create_info = ffi::VmaVirtualAllocationCreateInfo {
            size,
            alignment: alignment.into().unwrap_or(vk::DeviceSize::default()),
            flags: if let Some(flags_value) = flags.into() {
                flags_value.bits
            } else {
                VirtualAllocationCreateFlags::STRATEGY_MIN_TIME.bits
            },
            pUserData: p_user_data.into().unwrap_or(::std::ptr::null_mut()),
        };

        let mut vma_vallocation: ffi::VmaVirtualAllocation = unsafe { mem::zeroed() };
        let mut p_offset: vk::DeviceSize = unsafe { mem::zeroed() };
        unsafe {
            ffi_to_result(ffi::vmaVirtualAllocate(
                self.internal,
                &valloc_create_info,
                &mut vma_vallocation,
                &mut p_offset,
            ))?
        };

        Ok(vma_vallocation)
    }

    /// Frees virtual allocation inside given #VmaVirtualBlock.
    ///
    /// It is correct to call this function with `allocation == VK_NULL_HANDLE` - it does nothing.
    pub fn free(&mut self, allocation: VirtualAllocation) {
        unsafe { ffi::vmaVirtualFree(self.internal, allocation) };
    }

    /// Frees all virtual allocations inside given #VmaVirtualBlock.
    ///
    /// You must either call this function or free each virtual allocation individually with vmaVirtualFree()
    /// before destroying a virtual block. Otherwise, an assert is called.
    ///
    /// If you keep pointer to some additional metadata associated with your virtual allocation in its `pUserData`,
    /// don't forget to free it as well.
    pub fn clear(&mut self) {
        unsafe { ffi::vmaClearVirtualBlock(self.internal) };
    }

    /// Changes custom pointer associated with given virtual allocation.
    pub fn set_allocation_virtual_data(
        &mut self,
        allocation: VirtualAllocation,
        p_user_data: *mut ::std::os::raw::c_void,
    ) {
        unsafe { ffi::vmaSetVirtualAllocationUserData(self.internal, allocation, p_user_data) };
    }

    /// Calculates and returns statistics about virtual allocations and memory usage in given #VmaVirtualBlock.
    ///
    /// This function is fast to call. For more detailed statistics, see vmaCalculateVirtualBlockStatistics().
    pub fn get_statistics(&self) -> Statistics {
        let mut vma_stats: ffi::VmaStatistics = unsafe { mem::zeroed() };
        unsafe { ffi::vmaGetVirtualBlockStatistics(self.internal, &mut vma_stats) };
        vma_stats.into()
    }

    /// Calculates and returns detailed statistics about virtual allocations and memory usage in given #VmaVirtualBlock.
    ///
    /// This function is slow to call. Use for debugging purposes.
    /// For less detailed statistics, see vmaGetVirtualBlockStatistics().
    pub fn calculate_statistics(&self) -> DetailedStatistics {
        let mut vma_stats: ffi::VmaDetailedStatistics = unsafe { mem::zeroed() };
        unsafe { ffi::vmaCalculateVirtualBlockStatistics(self.internal, &mut vma_stats) };
        vma_stats.into()
    }

    /// Builds and returns a String in JSON format with information about given #VmaVirtualBlock.
    /// virtualBlock Virtual block.
    /// ppStatsString Returned string.
    /// detailedMap Pass `VK_FALSE` to only obtain statistics as returned by vmaCalculateVirtualBlockStatistics(). Pass `VK_TRUE` to also obtain full list of allocations and free spaces.
    pub fn build_stats_string(&self, detailed_map: bool) -> VkResult<String> {
        let mut stats_string: *mut ::std::os::raw::c_char = ::std::ptr::null_mut();
        unsafe {
            ffi::vmaBuildVirtualBlockStatsString(
                self.internal,
                &mut stats_string,
                if detailed_map { 1 } else { 0 },
            )
        };

        Ok(if stats_string.is_null() {
            String::new()
        } else {
            unsafe {
                let result = std::ffi::CStr::from_ptr(stats_string)
                    .to_string_lossy()
                    .into_owned();
                ffi::vmaFreeVirtualBlockStatsString(self.internal, stats_string);
                result
            }
        })
    }
}

/// Construct `AllocatorCreateFlags` with default values
impl Default for AllocatorCreateFlags {
    fn default() -> Self {
        AllocatorCreateFlags::NONE
    }
}

/// Construct `AllocationCreateInfo` with default values
impl Default for AllocationCreateInfo {
    fn default() -> Self {
        AllocationCreateInfo {
            flags: AllocationCreateFlags::NONE,
            usage: MemoryUsage::Unknown,
            required_flags: ash::vk::MemoryPropertyFlags::empty(),
            preferred_flags: ash::vk::MemoryPropertyFlags::empty(),
            memory_type_bits: 0,
            pool: None,
            p_user_data: ::std::ptr::null_mut(),
            priority: 0.0,
        }
    }
}

/// Construct `AllocatorPoolCreateInfo` with default values
impl Default for AllocatorPoolCreateInfo {
    fn default() -> Self {
        AllocatorPoolCreateInfo {
            memory_type_index: 0,
            flags: AllocatorPoolCreateFlags::NONE,
            block_size: 0,
            min_block_count: 0,
            max_block_count: 0,
            priority: 0.0,
            min_allocation_alignment: 0,
            p_memory_allocate_next: ::std::ptr::null_mut(),
        }
    }
}

/// Construct `DefragmentationInfo` with default values
impl Default for DefragmentationInfo {
    fn default() -> Self {
        DefragmentationInfo {
            flags: DefragmentationFlags::ALGORITHM_BALANCED,
            pool: None,
            max_bytes_per_pass: ash::vk::WHOLE_SIZE,
            max_allocations_per_pass: std::u32::MAX,
        }
    }
}

/// Custom `Drop` implementation to clean up internal allocation instance
impl Drop for Allocator {
    fn drop(&mut self) {
        unsafe {
            self.destroy();
        }
    }
}
