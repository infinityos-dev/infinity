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

use crate::arch::memory;
use crate::trace;
use acpic::{AcpiHandler, PhysicalMapping};
use core::ptr::NonNull;

#[derive(Copy, Clone)]
pub struct KernelAcpiHandler {}

impl AcpiHandler for KernelAcpiHandler {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> PhysicalMapping<Self, T> {
        let virt_addr = memory::hhdm_offset() + physical_address as u64;

        trace!(
            "Mapping physical address 0x{:x} (size: {}) to virtual address 0x{:x}",
            physical_address,
            size,
            virt_addr.as_u64()
        );

        let ptr = NonNull::new(virt_addr.as_mut_ptr()).unwrap_or_else(|| {
            panic!(
                "Failed to map physical address: 0x{:x} (virt: 0x{:x})",
                physical_address,
                virt_addr.as_u64()
            );
        });

        unsafe { PhysicalMapping::new(physical_address, ptr, size, size, self.clone()) }
    }

    fn unmap_physical_region<T>(_region: &PhysicalMapping<Self, T>) {
        // No-op: Static mapping via HHDM
    }
}
