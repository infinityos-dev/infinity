use acpic::AcpiHandler;

use crate::arch::memory::hhdm::HhdmOffset;

#[derive(Debug, Clone)]
struct KernelAcpiHandler {
    hhdm_offset: HhdmOffset,
}

impl AcpiHandler for KernelAcpiHandler {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> acpic::PhysicalMapping<Self, T> {
        todo!()
    }

    fn unmap_physical_region<T>(region: &acpic::PhysicalMapping<Self, T>) {
        todo!()
    }
}
