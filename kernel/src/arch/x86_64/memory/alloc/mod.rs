use core::{mem::MaybeUninit, slice};

use limine::{memory_map::EntryType, response::MemoryMapResponse};
use talc::{ErrOnOom, Talc, Talck};

const GLOBAL_ALLOCATOR_SIZE: u64 = 4 * 0x400 * 0x400; // 4 MiB

// This tells Rust that global allocations will use this static variable's allocation functions
#[global_allocator]
static GLOBAL_ALLOCATOR: Talck<spin::Mutex<()>, ErrOnOom> = Talck::new(Talc::new(ErrOnOom));

pub fn init(memory_map: &'static MemoryMapResponse, hhdm_offset: super::hhdm::HhdmOffset) {
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
}
