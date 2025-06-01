pub mod dummy;
pub mod linked_list;

#[global_allocator]
pub static GLOBAL_ALLOCATOR: linked_list::LockedHeap = linked_list::LockedHeap::empty();
