use crate::arch::memory::MEMORY;
use acpic::AcpiHandler;
use acpic::AcpiTables;
use acpic::InterruptModel;
use core::ops::DerefMut;
use raw_cpuid::CpuId;
use spin::Once;
use x86_64c::PhysAddr;
use x86_64c::VirtAddr;
use x86_64c::structures::paging::PageTableFlags;
use x86_64c::structures::paging::PhysFrame;
use x86_64c::structures::paging::Size4KiB;

#[derive(Debug)]
pub enum LocalApicAccess {
    /// No MMIO needed because x2apic uses register based configuration
    RegisterBased,
    /// The pointer to the mapped Local APIC
    Mmio(VirtAddr),
}

pub static LOCAL_APIC_ACCESS: Once<LocalApicAccess> = Once::new();

/// Maps the Local APIC memory if needed, and initializes LOCAL_APIC_ACCESS
pub fn map_if_needed(acpi_tables: &AcpiTables<impl AcpiHandler>) {
    LOCAL_APIC_ACCESS.call_once(|| {
        if CpuId::new().get_feature_info().unwrap().has_x2apic() {
            LocalApicAccess::RegisterBased
        } else {
            let platform_info = acpi_tables.platform_info().unwrap();
            let apic = match platform_info.interrupt_model {
                InterruptModel::Apic(apic) => apic,
                interrupt_model => panic!("Unknown interrupt model: {:#?}", interrupt_model),
            };
            let addr = PhysAddr::new(apic.local_apic_address);
            // Local APIC is always exactly 4 KiB, aligned to 4 KiB
            let frame = PhysFrame::<Size4KiB>::from_start_address(addr).unwrap();
            let memory = MEMORY.get().unwrap();
            let mut physical_memory = memory.physical_memory.lock();
            let mut virtual_memory = memory.virtual_memory.lock();
            let mut pages = virtual_memory.allocate_contiguous_pages(1).unwrap();
            let page = *pages.range().start();
            // Safety: We map to the correct page for the Local APIC
            unsafe {
                pages.map_to(
                    page,
                    frame,
                    PageTableFlags::PRESENT
                        | PageTableFlags::WRITABLE
                        | PageTableFlags::NO_CACHE
                        | PageTableFlags::NO_EXECUTE,
                    physical_memory.deref_mut(),
                )
            };
            LocalApicAccess::Mmio(page.start_address())
        }
    });
}
