use alloc::boxed::Box;
use limine::response::RsdpResponse;

use crate::{arch::limine::RSDP_REQUEST, trace};

pub mod handler;

pub unsafe fn get_acpi_tables(rsdp: &RsdpResponse) -> acpic::AcpiTables<impl acpic::AcpiHandler> {
    let address = rsdp.address();
    unsafe { acpic::AcpiTables::from_rsdp(handler::KernelAcpiHandler {}, address) }.unwrap()
}

pub fn init() {
    let rsdp = RSDP_REQUEST.get_response().unwrap();
    // Safety: We're not sending this across CPUs
    let acpi_tables = unsafe { get_acpi_tables(rsdp) };
    let signatures = acpi_tables
        .headers()
        .map(|header| header.signature)
        .collect::<Box<[_]>>();
    trace!("ACPI Tables: {signatures:?}");
    super::interrupts::apic::map_if_needed(&acpi_tables);
}
