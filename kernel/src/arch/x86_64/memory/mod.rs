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

use crate::{arch::memory::hhdm::HhdmOffset, trace};
use limine::response::MemoryMapResponse;

pub mod alloc;
pub mod hhdm;

pub fn init(memory_map: &'static MemoryMapResponse, hhdm_offset: HhdmOffset) {
    trace!("Hhdm Offset: {:#?}", hhdm_offset);
    self::alloc::init(memory_map, hhdm_offset);
    trace!("Memory initialized");
}
