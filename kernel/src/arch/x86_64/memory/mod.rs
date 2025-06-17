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

use crate::{
    arch::memory::{pmm::PhysicalMemory, vmm::VirtualMemory},
    trace,
};
use limine::response::MemoryMapResponse;
use spin::{Mutex, Once};
use x86_64c::{
    registers::control::Cr3Flags,
    structures::paging::{PhysFrame, Size4KiB},
};

pub mod alloc;
pub mod hhdm;
pub mod pmm;
pub mod vmm;

pub static MEMORY: Once<Memory> = Once::new();

#[non_exhaustive]
pub struct Memory {
    pub physical_memory: spin::Mutex<PhysicalMemory>,
    pub virtual_memory: spin::Mutex<VirtualMemory>,
    pub new_kernel_cr3: PhysFrame<Size4KiB>,
    pub new_kernel_cr3_flags: Cr3Flags,
}

pub fn init(memory_map: &'static MemoryMapResponse, hhdm_offset: hhdm::HhdmOffset) {
    let global_allocator_physical_start: u64 = self::alloc::init(memory_map, hhdm_offset);
    let physical_memory: pmm::PhysicalMemory = pmm::init(
        memory_map,
        global_allocator_physical_start,
        self::alloc::GLOBAL_ALLOCATOR_SIZE,
    );
    let vmm_return = vmm::init(physical_memory.clone(), hhdm_offset, memory_map);
    MEMORY.call_once(|| Memory {
        physical_memory: Mutex::new(physical_memory),
        virtual_memory: Mutex::new(vmm_return.virtual_memory),
        new_kernel_cr3: vmm_return.new_l4_frame,
        new_kernel_cr3_flags: vmm_return.cr3_flags,
    });
    trace!("Memory initialized");
}
