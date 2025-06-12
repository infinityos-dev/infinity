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

use crate::trace;
use limine::response::MemoryMapResponse;

pub mod alloc;
pub mod hhdm;
pub mod pmm;
pub mod vmm;

pub fn init(memory_map: &'static MemoryMapResponse, hhdm_offset: hhdm::HhdmOffset) {
    let global_allocator_physical_start: u64 = self::alloc::init(memory_map, hhdm_offset);
    let physical_memory: pmm::PhysicalMemory = pmm::init(
        memory_map,
        global_allocator_physical_start,
        self::alloc::GLOBAL_ALLOCATOR_SIZE,
    );
    vmm::init(physical_memory, hhdm_offset, memory_map);
    trace!("Memory initialized");
}
