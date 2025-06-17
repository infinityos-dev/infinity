use limine::{memory_map::EntryType, response::MemoryMapResponse};
use nodit::{Interval, NoditMap};
use x86_64c::{
    PhysAddr,
    structures::paging::{FrameAllocator, PageSize, PhysFrame},
};

use crate::{debug, trace};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum KernelMemoryUsageType {
    /// The memory region is used by the custom page tables, and as such is permanently reserved.
    PageTables,
    /// The memory region is used by the global heap, and as such is permanently reserved.
    GlobalAllocatorHeap,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum MemoryType {
    /// The memory region is freely usable.
    Usable,
    /// The memory region is permanently reserved.
    Reserved,
    /// The memory region is currently used by ACPI, but can be reclaimed once ACPI structures are no longer needed.
    AcpiReclaimable,
    /// The memory region is permanently reserved by ACPI, and must not be used.
    AcpiNVS,
    /// The memory region is unusable due to physical damage or similar errors.
    BadMemory,
    /// The memory region is used by the bootloader, but can be reclaimed once all responses have been processed and will no longer be used.
    BootloaderReclaimable,
    /// The memory region is used by the executable and modules, and as such is permanently reserved.
    ExecutableAndModules,
    /// The memory region is used by the framebuffer, and as such is permanently reserved.
    Framebuffer,
    /// The memory region is used by the kernels internal structures, and as such is permanently reserved.
    UsedByKernel(KernelMemoryUsageType),
}

#[derive(Clone)]
pub struct PhysicalMemory {
    pub(super) map: NoditMap<u64, Interval<u64>, MemoryType>,
}

pub fn init(
    memory_map: &'static MemoryMapResponse,
    global_allocator_physical_start: u64,
    global_allocator_size: u64,
) -> PhysicalMemory {
    let mem = PhysicalMemory {
        map: {
            let mut map = NoditMap::default();
            // We start with the state when Limine booted our kernel
            for entry in memory_map.entries() {
                let should_insert = match entry.entry_type {
                    EntryType::USABLE => Some(MemoryType::Usable),
                    EntryType::RESERVED => Some(MemoryType::Reserved),
                    EntryType::ACPI_RECLAIMABLE => Some(MemoryType::AcpiReclaimable),
                    EntryType::ACPI_NVS => Some(MemoryType::AcpiNVS),
                    EntryType::BAD_MEMORY => Some(MemoryType::BadMemory),
                    EntryType::BOOTLOADER_RECLAIMABLE => Some(MemoryType::BootloaderReclaimable),
                    EntryType::EXECUTABLE_AND_MODULES => Some(MemoryType::ExecutableAndModules),
                    EntryType::FRAMEBUFFER => Some(MemoryType::Framebuffer),
                    _ => {
                        // The entry might overlap, so let's not add it
                        None
                    }
                };
                if let Some(memory_type) = should_insert {
                    map
                        // Although they are guaranteed to not overlap and be ascending, Limine
                        // doesn't specify that they aren't guaranteed to not be touching even if they are the same.
                        .insert_merge_touching_if_values_equal(
                            (entry.base..entry.base + entry.length).into(),
                            memory_type,
                        )
                        .unwrap();
                }
            }
            // We track the memory used for the global allocator
            let _ = map.insert_overwrite(
                (global_allocator_physical_start
                    ..=global_allocator_physical_start + (global_allocator_size - 1))
                    .into(),
                MemoryType::UsedByKernel(KernelMemoryUsageType::GlobalAllocatorHeap),
            );
            map
        },
    };

    trace!(
        "Physical memory map initialized with {} entries",
        mem.map.len()
    );

    mem
}

unsafe impl<S: PageSize> FrameAllocator<S> for PhysicalMemory {
    fn allocate_frame(&mut self) -> Option<PhysFrame<S>> {
        let aligned_start = self.map.iter().find_map(|(interval, memory_type)| {
            if let MemoryType::Usable = memory_type {
                let aligned_start = interval.start().next_multiple_of(S::SIZE);
                let required_end_inclusive = aligned_start + (S::SIZE - 1);
                if required_end_inclusive <= interval.end() {
                    Some(aligned_start)
                } else {
                    None
                }
            } else {
                None
            }
        })?;
        let _ = self.map.insert_overwrite(
            (aligned_start..=aligned_start + (S::SIZE - 1)).into(),
            MemoryType::UsedByKernel(KernelMemoryUsageType::PageTables),
        );
        debug!("Allocated frame at {:#x}", aligned_start);
        Some(PhysFrame::from_start_address(PhysAddr::new(aligned_start)).unwrap())
    }
}
