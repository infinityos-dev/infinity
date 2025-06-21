use limine::{memory_map::EntryType, response::MemoryMapResponse};
use nodit::{Interval, NoditMap};
use x86_64c::{
    PhysAddr,
    structures::paging::{FrameAllocator, PageSize, PhysFrame},
};

use crate::{arch::memory::alloc::GLOBAL_ALLOCATOR_SIZE, debug};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum MemoryUsage {
    Usable,
    Reserved,
    AcpiReclaimable,
    AcpiNvs,
    BadMemory,
    BootloaderReclaimable,
    ExecutableAndModules,
    Framebuffer,
    Kernel(KernelMemoryUsage),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum KernelMemoryUsage {
    PageTables,
    GlobalHeap,
}

impl From<EntryType> for MemoryUsage {
    #[allow(deprecated)]
    fn from(entry_type: EntryType) -> Self {
        match entry_type {
            e if e == EntryType::USABLE => MemoryUsage::Usable,
            e if e == EntryType::RESERVED => MemoryUsage::Reserved,
            e if e == EntryType::ACPI_RECLAIMABLE => MemoryUsage::AcpiReclaimable,
            e if e == EntryType::ACPI_NVS => MemoryUsage::AcpiNvs,
            e if e == EntryType::BAD_MEMORY => MemoryUsage::BadMemory,
            e if e == EntryType::BOOTLOADER_RECLAIMABLE => MemoryUsage::BootloaderReclaimable,
            e if e == EntryType::EXECUTABLE_AND_MODULES || e == EntryType::KERNEL_AND_MODULES => {
                MemoryUsage::ExecutableAndModules
            }
            e if e == EntryType::FRAMEBUFFER => MemoryUsage::Framebuffer,
            _ => MemoryUsage::Reserved,
        }
    }
}

pub struct PhysicalMemory {
    pub map: NoditMap<u64, Interval<u64>, MemoryUsage>,
}

pub fn init(
    memory_map: &'static MemoryMapResponse,
    global_allocator_physical_start: u64,
) -> PhysicalMemory {
    PhysicalMemory {
        map: {
            let mut map = NoditMap::default();

            for entry in memory_map.entries() {
                let memory_usage: MemoryUsage = entry.entry_type.into();
                map.insert_merge_touching_if_values_equal(
                    (entry.base..entry.base + entry.length).into(),
                    memory_usage,
                )
                .unwrap();
            }

            let _ = map.insert_overwrite(
                (global_allocator_physical_start
                    ..=global_allocator_physical_start + (GLOBAL_ALLOCATOR_SIZE - 1))
                    .into(),
                MemoryUsage::Kernel(KernelMemoryUsage::GlobalHeap),
            );

            map
        },
    }
}

unsafe impl<S: PageSize> FrameAllocator<S> for PhysicalMemory {
    fn allocate_frame(&mut self) -> Option<PhysFrame<S>> {
        let aligned_start = self.map.iter().find_map(|(interval, memory_type)| {
            if let MemoryUsage::Usable = memory_type {
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
            MemoryUsage::Kernel(KernelMemoryUsage::PageTables),
        );
        debug!("Allocated new frame at: 0x{:.X}", aligned_start);
        Some(PhysFrame::from_start_address(PhysAddr::new(aligned_start)).unwrap())
    }
}
