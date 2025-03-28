use core::alloc::{GlobalAlloc, Layout};
use core::fmt::{self, Debug, Formatter};

extern crate std;
use std::sync::{Mutex, MutexGuard};

use crate::UnsafeStalloc;
use crate::align::*;

/// A wrapper around `UnsafeStalloc` that is safe to create because it prevents data races using a Mutex.
/// In comparison to `UnsafeStalloc`, the Mutex may cause a slight overhead.
pub struct SyncStalloc<const L: usize, const B: usize>
where
	Align<B>: Alignment,
{
	inner: Mutex<UnsafeStalloc<L, B>>,
}

impl<const L: usize, const B: usize> SyncStalloc<L, B>
where
	Align<B>: Alignment,
{
	/// Initializes a new empty `SyncStalloc` instance.
	pub const fn new() -> Self {
		Self {
			// SAFETY: The Mutex prevents concurrent access to the `UnsafeStalloc`.
			inner: Mutex::new(unsafe { UnsafeStalloc::<L, B>::new() }),
		}
	}

	/// Checks if the allocator is completely out of memory.
	/// If this is false, then you are guaranteed to be able to allocate
	/// a layout with a size and alignment of `B` bytes.
	/// This runs in O(1).
	pub fn is_oom(&self) -> bool {
		self.acquire_locked().is_oom()
	}

	/// Checks if the allocator is empty.
	/// If this is true, then you are guaranteed to be able to allocate
	/// a layout with a size of `B * L` bytes and an alignment of `B` bytes.
	/// If this is false, then this is guaranteed to be impossible.
	/// This runs in O(1).
	pub fn is_empty(&self) -> bool {
		self.acquire_locked().is_empty()
	}

	/// # Safety
	///
	/// Calling this function immediately invalidates all pointers into the allocator. Calling
	/// deallocate() with an invalidated pointer may result in the free list being corrupted.
	pub unsafe fn clear(&self) {
		// SAFETY: Upheld by the caller.
		unsafe { self.acquire_locked().clear() }
	}

	/// Tries to allocate `count` blocks. If the allocation succeed, a pointer is returned. This function
	/// never allocates more than necessary.
	///
	/// # Safety
	///
	/// `size` must be nonzero, and `align` must be a power of 2 in the range `1..=2^29 / B`.
	pub unsafe fn allocate_blocks(
		&self,
		size: usize,
		align: usize,
	) -> Result<NonNull<u8>, AllocError> {
		// SAFETY: Upheld by the caller.
		unsafe { self.acquire_locked().allocate_blocks(size, align) }
	}

	/// Deallocates a pointer.
	///
	/// # Safety
	///
	/// `ptr` must point to an allocation, and `size` must be the number of blocks
	/// in the allocation. That is, `size` is always in `1..=L`.
	pub unsafe fn deallocate_blocks(&self, ptr: NonNull<u8>, size: usize) {
		// SAFETY: Upheld by the caller.
		unsafe { self.acquire_locked().deallocate_blocks(ptr, size) }
	}

	/// Shrinks the allocation. This function always succeeds and never reallocates.
	///
	/// # Safety
	///
	/// `ptr` must point to a valid allocation of `old_size` blocks. `new_size` must be in `1..old_size`.
	pub unsafe fn shrink_in_place(&self, ptr: NonNull<u8>, old_size: usize, new_size: usize) {
		// SAFETY: Upheld by the caller.
		unsafe {
			self.acquire_locked()
				.shrink_in_place(ptr, old_size, new_size)
		}
	}

	/// Tries to grow the current allocation in-place. If that isn't possible, this function is a no-op.
	///
	/// # Safety
	///
	/// `ptr` must point to a valid allocation of `old_size` blocks. Also, `new_size > old_size`.
	pub unsafe fn grow_in_place(
		&self,
		ptr: NonNull<u8>,
		old_size: usize,
		new_size: usize,
	) -> Result<(), AllocError> {
		// SAFETY: Upheld by the caller.
		unsafe { self.acquire_locked().grow_in_place(ptr, old_size, new_size) }
	}

	fn acquire_locked(&self) -> MutexGuard<UnsafeStalloc<L, B>> {
		// Note: if this Mutex is poisoned, it means that one of the allocation functions panicked,
		// which is already declared to be UB. Therefore, we can assume that this is never poisoned.
		unsafe { self.inner.lock().unwrap_unchecked() }
	}
}

impl<const L: usize, const B: usize> Default for SyncStalloc<L, B>
where
	Align<B>: Alignment,
{
	fn default() -> Self {
		Self::new()
	}
}

impl<const L: usize, const B: usize> Debug for SyncStalloc<L, B>
where
	Align<B>: Alignment,
{
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "{:?}", self.acquire_locked())
	}
}

unsafe impl<const L: usize, const B: usize> GlobalAlloc for SyncStalloc<L, B>
where
	Align<B>: Alignment,
{
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		// SAFETY: upheld by the caller.
		unsafe { self.acquire_locked().alloc(layout) }
	}

	unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
		// SAFETY: upheld by the caller.
		unsafe { self.acquire_locked().alloc_zeroed(layout) }
	}

	unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
		// SAFETY: upheld by the caller.
		unsafe { self.acquire_locked().dealloc(ptr, layout) }
	}

	unsafe fn realloc(&self, ptr: *mut u8, old_layout: Layout, new_size: usize) -> *mut u8 {
		// SAFETY: upheld by the caller.
		unsafe { self.acquire_locked().realloc(ptr, old_layout, new_size) }
	}
}

#[cfg(feature = "allocator_api")]
use core::{
	alloc::{AllocError, Allocator},
	ptr::NonNull,
};

#[cfg(feature = "allocator_api")]
unsafe impl<const L: usize, const B: usize> Allocator for SyncStalloc<L, B>
where
	Align<B>: Alignment,
{
	fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
		self.acquire_locked().allocate(layout)
	}

	unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
		// SAFETY: Upheld by the caller.
		unsafe {
			self.acquire_locked().deallocate(ptr, layout);
		}
	}

	fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
		self.acquire_locked().allocate_zeroed(layout)
	}

	unsafe fn grow(
		&self,
		ptr: NonNull<u8>,
		old_layout: Layout,
		new_layout: Layout,
	) -> Result<NonNull<[u8]>, std::alloc::AllocError> {
		// SAFETY: Upheld by the caller.
		unsafe { self.acquire_locked().grow(ptr, old_layout, new_layout) }
	}

	unsafe fn grow_zeroed(
		&self,
		ptr: NonNull<u8>,
		old_layout: Layout,
		new_layout: Layout,
	) -> Result<NonNull<[u8]>, std::alloc::AllocError> {
		// SAFETY: Upheld by the caller.
		unsafe {
			self.acquire_locked()
				.grow_zeroed(ptr, old_layout, new_layout)
		}
	}

	unsafe fn shrink(
		&self,
		ptr: NonNull<u8>,
		old_layout: Layout,
		new_layout: Layout,
	) -> Result<NonNull<[u8]>, std::alloc::AllocError> {
		// SAFETY: Upheld by the caller.
		unsafe { self.acquire_locked().shrink(ptr, old_layout, new_layout) }
	}

	fn by_ref(&self) -> &Self
	where
		Self: Sized,
	{
		self
	}
}
