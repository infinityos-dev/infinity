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

use core::{mem::MaybeUninit, slice};

use crate::trace;

use limine::{memory_map::EntryType, response::MemoryMapResponse};
use talc::{ErrOnOom, Talc, Talck};

pub mod hhdm;

const GLOBAL_ALLOCATOR_SIZE: u64 = 4 * 0x400 * 0x400; // 4 MiB

// This tells Rust that global allocations will use this static variable's allocation functions
#[global_allocator]
static GLOBAL_ALLOCATOR: Talck<spin::Mutex<()>, ErrOnOom> = Talck::new(Talc::new(ErrOnOom));

pub fn init(memory_map: &'static MemoryMapResponse, hhdm_offset: hhdm::HhdmOffset) {
    let global_allocator_physical_start = memory_map
        .entries()
        .iter()
        .find(|entry| {
            entry.entry_type == EntryType::USABLE && entry.length >= GLOBAL_ALLOCATOR_SIZE
        })
        .unwrap()
        .base;

    let global_allocator_mem = unsafe {
        slice::from_raw_parts_mut::<MaybeUninit<u8>>(
            (u64::from(hhdm_offset) + global_allocator_physical_start) as *mut _,
            GLOBAL_ALLOCATOR_SIZE as usize,
        )
    };

    let mut talc = GLOBAL_ALLOCATOR.lock();
    let span = global_allocator_mem.into();
    unsafe { talc.claim(span) }.unwrap();

    trace!("Memory initialized");
}
