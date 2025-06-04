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

pub mod clock;
pub mod debug;
pub mod interrupts;
pub mod limine;
pub mod memory;
pub mod registers;
pub mod smp;

pub fn init() {
    assert!(limine::BASE_REVISION.is_supported());
    crate::writer::init();
    interrupts::init();
    memory::init(
        &limine::MEMMAP_REQUEST.get_response().unwrap(),
        limine::HHDM_REQUEST.get_response().unwrap().into(),
    );
    smp::init();
}
