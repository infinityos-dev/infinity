use crate::arch::memory::{hhdm::HhdmOffset, pmm::PhysicalMemory};
use core::fmt::Debug;
use core::mem::MaybeUninit;
use limine::{memory_map::EntryType, response::MemoryMapResponse};
use nodit::{Interval, NoditSet, interval::iu};
use raw_cpuid::CpuId;
use x86_64c::{
    PhysAddr, VirtAddr,
    registers::control::{Cr3, Cr3Flags},
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageSize, PageTable, PageTableFlags,
        PhysFrame, Size1GiB, Size2MiB, Size4KiB,
    },
};

pub struct VirtualMemory {
    pub set: NoditSet<u64, Interval<u64>>,
    pub cr3: PhysFrame<Size4KiB>,
    pub hhdm_offset: HhdmOffset,
}

pub struct VmmReturn {
    virt_mem: VirtualMemory,
    new_cr3: PhysFrame<Size4KiB>,
    new_cr3_flags: Cr3Flags,
}

pub fn init(
    mut physical_memory: PhysicalMemory,
    hhdm_offset: HhdmOffset,
    memory_map: &'static MemoryMapResponse,
) {
    let new_l4_frame = FrameAllocator::<Size4KiB>::allocate_frame(&mut physical_memory).unwrap();
    let new_l4_page_table =
        VirtAddr::new(u64::from(hhdm_offset) + new_l4_frame.start_address().as_u64())
            .as_mut_ptr::<MaybeUninit<PageTable>>();

    let new_l4_page_table = unsafe {
        new_l4_page_table
            .as_mut()
            .unwrap()
            // We initialize the page table to be blank
            .write(Default::default())
    };

    let mut new_offset_page_table =
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
            physical_memory,
            new_l4_frame,
        );
    } else {
        init_with_page_size::<Size2MiB>(
            memory_map,
            hhdm_offset,
            new_offset_page_table,
            physical_memory,
            new_l4_frame,
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
}

fn init_with_page_size<S: PageSize + Debug>(
    memory_map: &'static MemoryMapResponse,
    hhdm_offset: HhdmOffset,
    mut new_offset_page_table: OffsetPageTable,
    mut physical_memory: PhysicalMemory,
    new_l4_frame: PhysFrame,
) where
    for<'a> OffsetPageTable<'a>: Mapper<S>,
{
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
                                &mut physical_memory,
                            )
                            .unwrap()
                            .ignore()
                    };
                }
                last_mapped_address = Some(last_frame.start_address() + (S::SIZE - 1));
            }
        }
    }

    let virt_mem_return = VirtualMemory {
        set: {
            // Now let's keep track of the used virtual memory
            let mut set = NoditSet::default();
            // Let's add all of the offset mapped regions, keeping in mind we used 1 GiB pages
            for entry in memory_map.entries() {
                if [
                    EntryType::USABLE,
                    EntryType::BOOTLOADER_RECLAIMABLE,
                    EntryType::EXECUTABLE_AND_MODULES,
                    EntryType::FRAMEBUFFER,
                ]
                .contains(&entry.entry_type)
                {
                    let start = u64::from(hhdm_offset) + entry.base / S::SIZE * S::SIZE;
                    let end = u64::from(hhdm_offset)
                        + (entry.base + (entry.length - 1)) / S::SIZE * S::SIZE
                        + (S::SIZE - 1);
                    set.insert_merge_touching_or_overlapping((start..=end).into());
                }
            }
            // Let's add the top 512 GiB
            set.insert_merge_touching(iu(0xFFFFFF8000000000)).unwrap();
            set
        },
        cr3: new_l4_frame,
        hhdm_offset,
    };
}
