use core::fmt::{self, Debug, Formatter};
use std::alloc::{AllocError, Allocator, GlobalAlloc, Layout};
use std::ptr::{self, NonNull};

use crate::Stalloc;
use crate::align::*;

/// A wrapper around `Stalloc` that implements `Sync` and `GlobalAlloc`.
/// This type is unsafe to create, because it does not prevent data races.
/// Therefore, it is encouraged to only use it in single-threaded environments.
pub struct UnsafeStalloc<const L: usize, const B: usize>
where
	Align<B>: Alignment,
{
	inner: Stalloc<L, B>,
}

impl<const L: usize, const B: usize> Debug for UnsafeStalloc<L, B>
where
	Align<B>: Alignment,
{
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "{:?}", self.inner)
	}
}

impl<const L: usize, const B: usize> UnsafeStalloc<L, B>
where
	Align<B>: Alignment,
{
	/// # Safety
	///
	/// `UnsafeStalloc` does not prevent data races. It is strongly recommend
	/// to only use it in a single-threaded environment.
	pub const unsafe fn new() -> Self {
		Self {
			inner: Stalloc::<L, B>::new(),
		}
	}

	/// Checks if the allocator is completely out of memory.
	/// If this is false, then you are guaranteed to be able to allocate
	/// a layout with a size and alignment of `B` bytes.
	/// This runs in O(1).
	pub fn is_oom(&self) -> bool {
		self.inner.is_oom()
	}

	/// Checks if the allocator is empty.
	/// If this is true, then you are guaranteed to be able to allocate
	/// a layout with a size of `B * L` bytes and an alignment of `B` bytes.
	/// If this is false, then this is guaranteed to be impossible.
	/// This runs in O(1).
	pub fn is_empty(&self) -> bool {
		self.inner.is_empty()
	}

	/// # Safety
	///
	/// Calling this function immediately invalidates all pointers into the allocator. Calling
	/// deallocate() with an invalidated pointer may result in the free list being corrupted.
	pub unsafe fn clear(&self) {
		// SAFETY: Upheld by the caller.
		unsafe {
			self.inner.clear();
		}
	}
}

unsafe impl<const L: usize, const B: usize> Sync for UnsafeStalloc<L, B> where Align<B>: Alignment {}

unsafe impl<const L: usize, const B: usize> Allocator for UnsafeStalloc<L, B>
where
	Align<B>: Alignment,
{
	fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
		self.inner.allocate(layout)
	}

	unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
		// SAFETY: Upheld by the caller.
		unsafe { self.inner.deallocate(ptr, layout) }
	}

	fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
		self.inner.allocate_zeroed(layout)
	}

	unsafe fn grow(
		&self,
		ptr: NonNull<u8>,
		old_layout: Layout,
		new_layout: Layout,
	) -> Result<NonNull<[u8]>, std::alloc::AllocError> {
		// SAFETY: Upheld by the caller.
		unsafe { self.inner.grow(ptr, old_layout, new_layout) }
	}

	unsafe fn grow_zeroed(
		&self,
		ptr: NonNull<u8>,
		old_layout: Layout,
		new_layout: Layout,
	) -> Result<NonNull<[u8]>, std::alloc::AllocError> {
		// SAFETY: Upheld by the caller.
		unsafe { self.inner.grow_zeroed(ptr, old_layout, new_layout) }
	}

	unsafe fn shrink(
		&self,
		ptr: NonNull<u8>,
		old_layout: Layout,
		new_layout: Layout,
	) -> Result<NonNull<[u8]>, std::alloc::AllocError> {
		// SAFETY: Upheld by the caller.
		unsafe { self.inner.shrink(ptr, old_layout, new_layout) }
	}

	fn by_ref(&self) -> &Self
	where
		Self: Sized,
	{
		self
	}
}

unsafe impl<const L: usize, const B: usize> GlobalAlloc for UnsafeStalloc<L, B>
where
	Align<B>: Alignment,
{
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		self.inner
			.allocate(layout)
			.map(|p| p.as_ptr().cast())
			.unwrap_or(ptr::null_mut())
	}

	unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
		self.inner
			.allocate_zeroed(layout)
			.map(|p| p.as_ptr().cast())
			.unwrap_or(ptr::null_mut())
	}

	unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
		// SAFETY: upheld by the caller.
		unsafe {
			self.inner.deallocate(NonNull::new_unchecked(ptr), layout);
		}
	}

	unsafe fn realloc(&self, ptr: *mut u8, old_layout: Layout, new_size: usize) -> *mut u8 {
		// SAFETY: upheld by the caller.
		let new_layout = unsafe { Layout::from_size_align_unchecked(new_size, old_layout.align()) };

		if new_size > old_layout.size() {
			// SAFETY: upheld by the caller.
			unsafe {
				self.inner
					.grow(NonNull::new_unchecked(ptr), old_layout, new_layout)
					.map(|p| p.as_ptr().cast())
					.unwrap_or(ptr::null_mut())
			}
		} else {
			// SAFETY: upheld by the caller.
			// Note: if `new_size` == `old_layout.size()`, this should be a no-op.
			unsafe {
				self.inner
					.shrink(NonNull::new_unchecked(ptr), old_layout, new_layout)
					.map(|p| p.as_ptr().cast())
					.unwrap_or(ptr::null_mut())
			}
		}
	}
}
