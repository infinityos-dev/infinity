use crate::trace;
use limine::firmware_type::FirmwareType;

pub fn init() {
    trace!("Firmware type: {:?}", get_firmware_type());
}

pub fn get_firmware_type() -> &'static str {
    firmware_type_to_str(
        crate::arch::limine::FIRMWARE_TYPE_REQUEST
            .get_response()
            .unwrap()
            .firmware_type(),
    )
}

fn firmware_type_to_str(ft: FirmwareType) -> &'static str {
    if ft == FirmwareType::X86_BIOS {
        "X86_BIOS"
    } else if ft == FirmwareType::UEFI_32 {
        "UEFI_32"
    } else if ft == FirmwareType::UEFI_64 {
        "UEFI_64"
    } else if ft == FirmwareType::SBI {
        "SBI"
    } else {
        "UNKNOWN"
    }
}
