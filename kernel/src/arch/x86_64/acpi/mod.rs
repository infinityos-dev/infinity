use crate::arch::memory::hhdm::HhdmOffset;
use acpic::AcpiHandler;

#[derive(Debug, Clone)]
struct KernelAcpiHandler {
    _hhdm_offset: HhdmOffset,
}

impl AcpiHandler for KernelAcpiHandler {
    unsafe fn map_physical_region<T>(
        &self,
        _physical_address: usize,
        _size: usize,
    ) -> acpic::PhysicalMapping<Self, T> {
        todo!()
    }

    fn unmap_physical_region<T>(_region: &acpic::PhysicalMapping<Self, T>) {
        todo!()
    }
}
