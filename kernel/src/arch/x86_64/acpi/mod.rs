use acpic::{AcpiHandler, AcpiTables};
use alloc::boxed::Box;
use handler::KernelAcpiHandler;
use limine::response::RsdpResponse;

use crate::{arch::limine::RSDP_REQUEST, debug, trace};

pub mod apic;
pub mod handler;

pub fn init() {
    let rsdp = RSDP_REQUEST.get_response().unwrap();
    let acpi_tables = unsafe { get_acpi_tables(rsdp) };
    let signatures = acpi_tables
        .headers()
        .map(|header| header.signature)
        .collect::<Box<[_]>>();
    trace!("ACPI Tables: {signatures:?}");

    debug!(
        "ACPI platform information\n{:#?}",
        acpi_tables.platform_info().unwrap()
    );

    apic::init(&acpi_tables);
}

/// # Safety
/// You can store the returned value in CPU local data, but you cannot send it across CPUs because the other CPUs did not flush their cache for changes in page tables
pub unsafe fn get_acpi_tables(rsdp: &RsdpResponse) -> AcpiTables<impl AcpiHandler> {
    let address: usize = rsdp.address();
    unsafe { AcpiTables::from_rsdp(KernelAcpiHandler {}, address) }.unwrap()
}
