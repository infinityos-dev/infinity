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

use core::fmt::Debug;
use core::slice;
use core::{mem::MaybeUninit, ops::RangeInclusive};
use hhdm::HhdmOffset;
use limine::{memory_map::EntryType, response::MemoryMapResponse};
use pmm::InitialFrameAllocator;
use raw_cpuid::CpuId;
use x86_64c::registers::control::Cr3;
use x86_64c::{
    PhysAddr, VirtAddr,
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageSize, PageTable, PageTableFlags,
        PhysFrame, Size1GiB, Size2MiB, Size4KiB,
    },
};

use crate::trace;

pub mod hhdm;
pub mod pmm;

pub fn init(memory_map: &'static MemoryMapResponse, hhdm_offset: hhdm::HhdmOffset) {
    let global_allocator_size = {
        // 4 MiB
        4 * 0x400 * 0x400
    };
    let global_allocator_physical_start = memory_map
        .entries()
        .iter()
        .find(|entry| {
            entry.entry_type == EntryType::USABLE && entry.length >= global_allocator_size
        })
        .unwrap()
        .base;

    // Safety: No frames have been allocated yet
    let mut frame_allocator = unsafe {
        InitialFrameAllocator::new(
            memory_map,
            global_allocator_physical_start
                ..=global_allocator_physical_start + (global_allocator_size - 1),
        )
    };

    let new_l4_frame = frame_allocator.allocate_frame().unwrap();
    // Safety: The allocated frame is in usable memory, which is offset mapped
    let new_l4_page_table = unsafe {
        VirtAddr::new(u64::from(hhdm_offset) + new_l4_frame.start_address().as_u64())
            .as_mut_ptr::<MaybeUninit<PageTable>>()
            .as_mut()
            .unwrap()
            .write(Default::default())
    };

    let mut new_offset_page_table =
        unsafe { OffsetPageTable::new(new_l4_page_table, VirtAddr::new(hhdm_offset.into())) };

    // Offset map everything that is currently offset mapped
    let mut last_mapped_address = None::<PhysAddr>;
    for entry in memory_map.entries() {
        if [
            EntryType::USABLE,
            EntryType::BOOTLOADER_RECLAIMABLE,
            EntryType::EXECUTABLE_AND_MODULES,
            EntryType::FRAMEBUFFER,
        ]
        .contains(&entry.entry_type)
        {
            let range_to_map = {
                let first = PhysAddr::new(entry.base);
                let last = first + (entry.length - 1);
                match last_mapped_address {
                    Some(last_mapped_address) => {
                        if first > last_mapped_address {
                            Some(first..=last)
                        } else if last > last_mapped_address {
                            Some(last_mapped_address + 1u64..=last)
                        } else {
                            None
                        }
                    }
                    None => Some(first..=last),
                }
            };
            if let Some(range_to_map) = range_to_map {
                fn map<S: PageSize + Debug>(
                    range_to_map: RangeInclusive<PhysAddr>,
                    hhdm_offset: HhdmOffset,
                    last_mapped_address: &mut Option<PhysAddr>,
                    new_offset_page_table: &mut impl Mapper<S>,
                    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
                ) {
                    let first_frame = PhysFrame::<S>::containing_address(*range_to_map.start());
                    let last_frame = PhysFrame::<S>::containing_address(*range_to_map.end());
                    let page_count = last_frame - first_frame + 1;

                    for i in 0..page_count {
                        let frame = first_frame + i;
                        let page = Page::<S>::from_start_address(VirtAddr::new(
                            frame.start_address().as_u64() + u64::from(hhdm_offset),
                        ))
                        .unwrap();
                        unsafe {
                            new_offset_page_table
                                .map_to(
                                    page,
                                    frame,
                                    PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                                    frame_allocator,
                                )
                                .unwrap()
                                // Cache will be reloaded anyways when we change Cr3
                                .ignore()
                        };
                    }
                    *last_mapped_address = Some(last_frame.start_address() + (S::SIZE - 1));
                }
                if CpuId::new()
                    .get_extended_processor_and_feature_identifiers()
                    .unwrap()
                    .has_1gib_pages()
                {
                    map::<Size1GiB>(
                        range_to_map,
                        hhdm_offset,
                        &mut last_mapped_address,
                        &mut new_offset_page_table,
                        &mut frame_allocator,
                    );
                } else {
                    map::<Size2MiB>(
                        range_to_map,
                        hhdm_offset,
                        &mut last_mapped_address,
                        &mut new_offset_page_table,
                        &mut frame_allocator,
                    );
                }
            }
        }
    }

    // We must map the kernel, which lies in the top 2 GiB of virtual memory. We can just reuse Limine's mappings for the top 512 GiB
    let (current_l4_frame, cr3_flags) = Cr3::read();
    let current_l4_page_table = unsafe {
        VirtAddr::new(u64::from(hhdm_offset) + current_l4_frame.start_address().as_u64())
            .as_mut_ptr::<PageTable>()
            .as_mut()
            .unwrap()
    };
    new_l4_page_table[511].clone_from(&current_l4_page_table[511]);

    // Safety: Everything that needs to be mapped is mapped
    unsafe { Cr3::write(new_l4_frame, cr3_flags) };

    // Safety: We've reserved the physical memory and it is already offset mapped
    let global_allocator_mem = unsafe {
        slice::from_raw_parts_mut(
            (u64::from(hhdm_offset) + global_allocator_physical_start) as *mut _,
            global_allocator_size as usize,
        )
    };
    super::alloc::GLOBAL_ALLOCATOR
        .lock()
        .init_from_slice(global_allocator_mem);

    trace!("Memory initialized");
}

/* Get functions */
