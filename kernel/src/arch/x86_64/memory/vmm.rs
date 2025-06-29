use crate::{
    arch::memory::{hhdm::HhdmOffset, pmm::PhysicalMemory},
    error, trace,
};
use core::mem::MaybeUninit;
use core::{fmt::Debug, ops::RangeInclusive};
use limine::{memory_map::EntryType, response::MemoryMapResponse};
use nodit::{Interval, NoditSet, interval::iu};
use raw_cpuid::CpuId;
use x86_64c::structures::idt::InterruptStackFrame;
use x86_64c::structures::idt::PageFaultErrorCode;
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
    pub virt_mem: VirtualMemory,
    pub new_cr3: PhysFrame<Size4KiB>,
    pub new_cr3_flags: Cr3Flags,
}

pub fn init(
    mut physical_memory: PhysicalMemory,
    hhdm_offset: HhdmOffset,
    memory_map: &'static MemoryMapResponse,
) -> VmmReturn {
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

    let new_offset_page_table =
        unsafe { OffsetPageTable::new(new_l4_page_table, VirtAddr::new(hhdm_offset.into())) };

    let vmm_return: VirtualMemory;

    if CpuId::new()
        .get_extended_processor_and_feature_identifiers()
        .unwrap()
        .has_1gib_pages()
    {
        trace!("1 GiB pages are supported");
        vmm_return = init_with_page_size::<Size1GiB>(
            memory_map,
            hhdm_offset,
            new_offset_page_table,
            physical_memory,
            new_l4_frame,
        );
    } else {
        trace!("2 MiB pages are supported");
        vmm_return = init_with_page_size::<Size2MiB>(
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

    VmmReturn {
        virt_mem: vmm_return,
        new_cr3: new_l4_frame,
        new_cr3_flags: cr3_flags,
    }
}

fn init_with_page_size<S: PageSize + Debug>(
    memory_map: &'static MemoryMapResponse,
    hhdm_offset: HhdmOffset,
    mut new_offset_page_table: OffsetPageTable,
    mut physical_memory: PhysicalMemory,
    new_l4_frame: PhysFrame,
) -> VirtualMemory
where
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

    VirtualMemory {
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
    }
}

impl VirtualMemory {
    /// Returns the start page of the allocated range of pages.
    /// Pages are guaranteed not to be mapped.
    pub fn allocate_contiguous_pages<S: PageSize + Debug>(
        &'_ mut self,
        n_pages: u64,
    ) -> Option<AllocatedPages<'_, S>> {
        let start_page = Page::<S>::from_start_address(VirtAddr::new({
            let range = self
                .set
                .gaps_trimmed(iu(0xffff800000000000))
                .find_map(|gap| {
                    let aligned_start = gap.start().next_multiple_of(S::SIZE);
                    let required_end_inclusive = aligned_start + (n_pages * S::SIZE - 1);
                    if required_end_inclusive <= gap.end() {
                        Some(aligned_start..=required_end_inclusive)
                    } else {
                        None
                    }
                })?;
            let start = *range.start();
            self.set
                .insert_merge_touching(Interval::from(range))
                .unwrap();
            start
        }))
        .unwrap();
        Some(AllocatedPages {
            virtual_memory: self,
            range: start_page..=start_page + (n_pages - 1),
        })
    }

    /// # Safety
    /// The pages must have been allocated by [`VirtualMemory`]
    pub unsafe fn already_allocated<S: PageSize>(
        &mut self,
        pages: RangeInclusive<Page<S>>,
    ) -> AllocatedPages<'_, S> {
        AllocatedPages {
            virtual_memory: self,
            range: pages,
        }
    }
}

pub struct AllocatedPages<'a, S: PageSize> {
    virtual_memory: &'a mut VirtualMemory,
    range: RangeInclusive<Page<S>>,
}

impl<S: PageSize> AllocatedPages<'_, S> {
    pub fn range(&self) -> &RangeInclusive<Page<S>> {
        &self.range
    }

    fn get_offset_page_table(&self) -> OffsetPageTable<'_> {
        let level_4_page_table = VirtAddr::new(
            u64::from(self.virtual_memory.hhdm_offset)
                + self.virtual_memory.cr3.start_address().as_u64(),
        )
        .as_mut_ptr::<PageTable>();
        // Safety: We can access it through HHDM
        let level_4_page_table = unsafe { level_4_page_table.as_mut() }.unwrap();
        // Safety: No other code is currently modifying page tables
        unsafe {
            OffsetPageTable::new(
                level_4_page_table,
                VirtAddr::new(self.virtual_memory.hhdm_offset.into()),
            )
        }
    }

    /// # Safety
    /// See the safety for [`x86_64::structures::paging::mapper::Mapper::map_to`]
    pub unsafe fn map_to(
        &mut self,
        page: Page<S>,
        frame: PhysFrame<S>,
        flags: PageTableFlags,
        frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    ) where
        S: Debug,
        for<'a> OffsetPageTable<'a>: Mapper<S>,
    {
        if self.range.contains(&page) {
            let mut offset_page_table = self.get_offset_page_table();
            // Safety: same as this function's safety, plus we ensure that the page we are mapping is allocated properly
            unsafe { offset_page_table.map_to(page, frame, flags, frame_allocator) }
                .unwrap()
                .flush();
        } else {
            panic!(
                "Tried to map page {page:?}, which is outside of allocated range {:?}",
                self.range
            )
        }
    }

    /// All pages must be mapped
    pub fn unmap_and_deallocate(self)
    where
        for<'a> OffsetPageTable<'a>: Mapper<S>,
    {
        let pages = self.range.clone();
        let mut offset_page_table = self.get_offset_page_table();
        for page in pages.clone() {
            offset_page_table.unmap(page).unwrap().1.flush();
        }
        let _ = self.virtual_memory.set.cut({
            let start = pages.start().start_address().as_u64();
            let end_inclusive = pages.end().start_address().as_u64() + (S::SIZE - 1);
            Interval::from(start..=end_inclusive)
        });
    }
}

pub extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64c::registers::control::Cr2;

    error!("EXCEPTION: PAGE FAULT");
    error!("Accessed Address: {:?}", Cr2::read());
    error!("Error Code: {:?}\n{:#?}\n", error_code, stack_frame);
    crate::hal::halt_loop();
}
