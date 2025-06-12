use crate::{arch::memory::hhdm::HhdmOffset, trace};
use core::fmt::Debug;
use core::mem::MaybeUninit;
use limine::{memory_map::EntryType, response::MemoryMapResponse};
use raw_cpuid::CpuId;
use x86_64c::{
    PhysAddr, VirtAddr,
    registers::control::Cr3,
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageSize, PageTable, PageTableFlags,
        PhysFrame, Size1GiB, Size2MiB, Size4KiB,
    },
};

pub fn init(
    mut physical_memory: super::pmm::PhysicalMemory,
    hhdm_offset: HhdmOffset,
    memory_map: &'static MemoryMapResponse,
) {
    let new_l4_frame = FrameAllocator::<Size4KiB>::allocate_frame(&mut physical_memory).unwrap();
    // Safety: The allocated frame is in usable memory, which is offset mapped
    let new_l4_page_table =
        VirtAddr::new(u64::from(hhdm_offset) + new_l4_frame.start_address().as_u64())
            // We use `MaybeUninit` because the memory is uninitialized
            .as_mut_ptr::<MaybeUninit<PageTable>>();
    let new_l4_page_table = unsafe {
        new_l4_page_table
            .as_mut()
            .unwrap()
            // We initialize the page table to be blank
            .write(Default::default())
    };
    // Safety: We are only using usable memory, which is offset mapped
    let new_offset_page_table: OffsetPageTable<'_> =
        unsafe { OffsetPageTable::new(new_l4_page_table, VirtAddr::new(hhdm_offset.into())) };

    if CpuId::new()
        .get_extended_processor_and_feature_identifiers()
        .unwrap()
        .has_1gib_pages()
    {
        init_with_page_size::<Size1GiB>(
            memory_map,
            hhdm_offset,
            new_offset_page_table,
            &mut physical_memory,
        );
    } else {
        init_with_page_size::<Size2MiB>(
            memory_map,
            hhdm_offset,
            new_offset_page_table,
            &mut physical_memory,
        );
    }

    // We must map the kernel, which lies in the top 2 GiB of virtual memory
    // We can just reuse Limine's mappings for the top 512 GiB
    let (current_l4_frame, cr3_flags) = Cr3::read();
    let current_l4_page_table = unsafe {
        VirtAddr::new(u64::from(hhdm_offset) + current_l4_frame.start_address().as_u64())
            .as_mut_ptr::<PageTable>()
            .as_mut()
            .unwrap()
    };
    new_l4_page_table[511].clone_from(&current_l4_page_table[511]);

    unsafe { Cr3::write(new_l4_frame, cr3_flags) };

    trace!(
        "Virtual memory manager initialized with new L4 page table at {:?}",
        new_l4_frame.start_address()
    );
}

fn init_with_page_size<S: PageSize + Debug>(
    memory_map: &'static MemoryMapResponse,
    hhdm_offset: HhdmOffset,
    mut new_offset_page_table: OffsetPageTable<'_>,
    mut physical_memory: &mut super::pmm::PhysicalMemory,
) where
    for<'a> OffsetPageTable<'a>: Mapper<S>,
{
    // Offset map everything that is currently offset mapped
    let mut last_mapped_address = None::<PhysAddr>;
    for entry in memory_map.entries() {
        if [
            EntryType::USABLE,
            EntryType::BOOTLOADER_RECLAIMABLE,
            EntryType::EXECUTABLE_AND_MODULES,
            EntryType::FRAMEBUFFER,
        ]
        .contains(&entry.entry_type)
        {
            let range_to_map = {
                let first = PhysAddr::new(entry.base);
                let last = first + (entry.length - 1);
                match last_mapped_address {
                    Some(last_mapped_address) => {
                        if first > last_mapped_address {
                            Some(first..=last)
                        } else if last > last_mapped_address {
                            Some(last_mapped_address + 1u64..=last)
                        } else {
                            None
                        }
                    }
                    None => Some(first..=last),
                }
            };
            if let Some(range_to_map) = range_to_map {
                let first_frame = PhysFrame::<S>::containing_address(*range_to_map.start());
                let last_frame = PhysFrame::<S>::containing_address(*range_to_map.end());
                let page_count = last_frame - first_frame + 1;

                for i in 0..page_count {
                    let frame = first_frame + i;
                    let page = Page::<S>::from_start_address(VirtAddr::new(
                        frame.start_address().as_u64() + u64::from(hhdm_offset),
                    ))
                    .unwrap();
                    unsafe {
                        new_offset_page_table
                            .map_to(
                                page,
                                frame,
                                PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                                physical_memory,
                            )
                            .unwrap()
                            // Cache will be reloaded anyways when we change Cr3
                            .ignore()
                    };
                }
                last_mapped_address = Some(last_frame.start_address() + (S::SIZE - 1));
            }
        }
    }
}
