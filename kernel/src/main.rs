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

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(hexium_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::{panic::PanicInfo, sync::atomic::AtomicBool};
use hexium_os::init;

#[test_case]
fn test_example() {
    assert_eq!(1 + 1, 2);
}

#[unsafe(no_mangle)]
unsafe extern "C" fn kmain() -> ! {
    /*
        Issue#30: The lines at the end of this comment below do not seem to have an effect after the init method above
        however calling them above the init method causes a boot-loop.
        NOTE: Calling them after the init method after the executor code has been commented back in,
        will cause them not to be run as the executor code seems to block the 'thread'.
        print!("Test");
        println!("Test2");
    */

    init();

    #[cfg(test)]
    {
        test_main();
    }

    #[cfg(not(test))]
    hexium_os::hal::halt_loop();
    #[cfg(test)]
    loop {}
}

static DID_PANIC: AtomicBool = AtomicBool::new(false);

#[cfg(not(test))]
#[panic_handler]
/// Handles panics in production, detergates to rsod_handler
fn panic(info: &PanicInfo) -> ! {
    use core::sync::atomic::Ordering;

    match DID_PANIC.compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed) {
        Ok(_) => {
            use hexium_os::rsod::rsod_handler;
            rsod_handler(info);
        }
        Err(_) => {
            hexium_os::hal::halt_loop();
        }
    }
}

#[cfg(test)]
#[panic_handler]
/// Handles panics during binary tests, delegates to test_panic_handler
fn panic(info: &PanicInfo) -> ! {
    use core::sync::atomic::Ordering;

    match DID_PANIC.compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed) {
        Ok(_) => hexium_os::test_panic_handler(info),
        Err(_) => {
            hexium_os::hal::halt_loop();
        }
    }
}
