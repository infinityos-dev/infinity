use crate::arch::memory::MEMORY;
use acpic::{AcpiHandler, PhysicalMapping};
use core::{fmt::Debug, ops::DerefMut, ptr::NonNull};
use raw_cpuid::CpuId;
use x86_64c::{
    PhysAddr, VirtAddr,
    structures::paging::{
        Mapper, OffsetPageTable, Page, PageSize, PageTableFlags, PhysFrame, Size1GiB, Size2MiB,
    },
};

#[derive(Debug, Clone)]
pub struct KernelAcpiHandler {}

impl KernelAcpiHandler {
    fn map_physical_region_with_page_size<S: PageSize + Debug, T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> acpic::PhysicalMapping<Self, T>
    where
        for<'a> OffsetPageTable<'a>: Mapper<S>,
    {
        let memory = MEMORY.get().unwrap();
        let mut physical_memory = memory.physical_memory.lock();
        let mut virtual_memory = memory.virtual_memory.lock();
        let n_pages = ((size + physical_address) as u64).div_ceil(S::SIZE)
            - physical_address as u64 / S::SIZE;
        let start_frame =
            PhysFrame::<S>::containing_address(PhysAddr::new(physical_address as u64));
        let mut pages = virtual_memory.allocate_contiguous_pages(n_pages).unwrap();
        let start_page = *pages.range().start();

        for i in 0..n_pages {
            unsafe {
                pages.map_to(
                    start_page + i,
                    start_frame + i,
                    PageTableFlags::PRESENT | PageTableFlags::NO_EXECUTE,
                    physical_memory.deref_mut(),
                );
            }
        }

        unsafe {
            PhysicalMapping::new(
                physical_address,
                NonNull::new(
                    (start_page.start_address() + physical_address as u64 % S::SIZE).as_mut_ptr(),
                )
                .unwrap(),
                size,
                (n_pages * S::SIZE) as usize,
                self.clone(),
            )
        }
    }

    fn unmap_physical_region_with_page_size<S: PageSize + Debug, T>(
        region: &acpic::PhysicalMapping<Self, T>,
    ) where
        for<'a> OffsetPageTable<'a>: Mapper<S>,
    {
        let start_page =
            Page::<S>::containing_address(VirtAddr::new(region.virtual_start().as_ptr() as u64));
        let n_pages = region.mapped_length() as u64 / S::SIZE;
        let mut virtual_memory = MEMORY.get().unwrap().virtual_memory.lock();
        let pages = start_page..=start_page + (n_pages - 1);
        // Safety: this function will only be called with regions mapped by the `map_physical_region` function
        unsafe { virtual_memory.already_allocated(pages) }.unmap_and_deallocate();
    }
}

impl AcpiHandler for KernelAcpiHandler {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> acpic::PhysicalMapping<Self, T> {
        if CpuId::new()
            .get_extended_processor_and_feature_identifiers()
            .unwrap()
            .has_1gib_pages()
        {
            self.map_physical_region_with_page_size::<Size1GiB, T>(physical_address, size)
        } else {
            self.map_physical_region_with_page_size::<Size2MiB, T>(physical_address, size)
        }
    }

    fn unmap_physical_region<T>(region: &acpic::PhysicalMapping<Self, T>) {
        if CpuId::new()
            .get_extended_processor_and_feature_identifiers()
            .unwrap()
            .has_1gib_pages()
        {
            Self::unmap_physical_region_with_page_size::<Size1GiB, T>(region)
        } else {
            Self::unmap_physical_region_with_page_size::<Size2MiB, T>(region)
        }
    }
}
