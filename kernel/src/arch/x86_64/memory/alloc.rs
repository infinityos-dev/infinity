use crate::{arch::memory::hhdm::HhdmOffset, trace};
use core::{mem::MaybeUninit, slice};
use limine::{memory_map::EntryType, response::MemoryMapResponse};
use talc::{ErrOnOom, Talc, Talck};

#[global_allocator]
static GLOBAL_ALLOCATOR: Talck<spin::Mutex<()>, ErrOnOom> = Talck::new(Talc::new(ErrOnOom));

pub const GLOBAL_ALLOCATOR_SIZE: u64 = 2 * 1024 * 1024;

pub fn init(memory_map: &'static MemoryMapResponse, hhdm_offset: HhdmOffset) -> u64 {
    let allocator_physical_start = memory_map
        .entries()
        .iter()
        .find(|entry| {
            entry.entry_type == EntryType::USABLE && entry.length >= GLOBAL_ALLOCATOR_SIZE
        })
        .unwrap()
        .base;

    let allocator_memory = unsafe {
        slice::from_raw_parts_mut::<MaybeUninit<u8>>(
            (u64::from(hhdm_offset) + allocator_physical_start) as *mut _,
            GLOBAL_ALLOCATOR_SIZE as usize,
        )
    };

    {
        let mut talc = GLOBAL_ALLOCATOR.lock();
        let span = allocator_memory.into();
        unsafe { talc.claim(span) }.unwrap();
    }

    trace!(
        "Talck Physical start address:\t0x{:x}",
        allocator_physical_start + hhdm_offset
    );
    trace!("Talck allocator size:\t\t0x{:x}", GLOBAL_ALLOCATOR_SIZE);

    allocator_physical_start
}
