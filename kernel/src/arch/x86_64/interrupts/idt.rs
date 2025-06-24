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

use super::InterruptIndex;
use super::gdt;
use crate::{debug, trace};
use lazy_static::lazy_static;
use x86_64c::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt.page_fault
            .set_handler_fn(super::super::memory::vmm::page_fault_handler);
        //FIXME: Need to unmask the interrupts for it to work
        idt[InterruptIndex::Timer.as_usize()]
            .set_handler_fn(super::super::clock::pit::interrupt_handler);
        idt[InterruptIndex::Keyboard.as_usize()]
            .set_handler_fn(crate::devices::keyboard::interrupt_handler);
        idt
    };
}

pub fn init() {
    IDT.load();
    trace!("IDT initialized");
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    debug!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}
