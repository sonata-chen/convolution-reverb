use std::alloc::{AllocError, Allocator, Global, Layout};
use std::ptr::NonNull;


#[derive(Copy, Clone, Default, Debug)]
pub struct AlignedAllocator<const ALIGNMENT: usize = 64>;

unsafe impl<const ALIGNMENT: usize> Allocator for AlignedAllocator<ALIGNMENT> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        Global.allocate(layout.align_to(ALIGNMENT).unwrap())
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe { Global.deallocate(ptr, layout.align_to(ALIGNMENT).unwrap()) }
    }
}

pub static ALIGNED: AlignedAllocator<64> = AlignedAllocator;
