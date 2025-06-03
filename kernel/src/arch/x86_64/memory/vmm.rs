/*
 * This file is part of Hexium OS.
 * Copyright (C) 2025 The Hexium OS Authors – see the AUTHORS file.
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

use crate::{error, print};
use x86_64c::structures::{
    idt::{InterruptStackFrame, PageFaultErrorCode},
    paging::OffsetPageTable,
};

pub extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64c::registers::control::Cr2;

    error!("EXCEPTION: PAGE FAULT");
    error!("Accessed Address: {:?}", Cr2::read());
    error!("Error Code: {:?}", error_code);
    print!("\n{:#?}\n", stack_frame);
    crate::hal::halt_loop();
}

pub fn initialize_offset_table() -> OffsetPageTable<'static> {
    unsafe {
        let level_4_table = super::paging::active_level_4_table();
        OffsetPageTable::new(level_4_table, super::hhdm_offset())
    }
}
