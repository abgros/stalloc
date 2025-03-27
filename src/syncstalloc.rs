use core::alloc::{AllocError, Allocator, GlobalAlloc, Layout};
use core::fmt::{self, Debug, Formatter};
use core::ptr::{self, NonNull};
use std::sync::Mutex;

use crate::Stalloc;
use crate::align::*;

/// A wrapper around `Stalloc` that implements `Sync` and `GlobalAlloc`.
/// This type is safe to create because it prevents data races using a Mutex.
/// In comparison to `UnsafeStalloc`, the Mutex may cause a slight overhead.
/// I don't think it's possible for this Mutex to be poisoned, but if it does,
/// all accesses to the allocator will fail.
pub struct SyncStalloc<const L: usize, const B: usize>
where
	Align<B>: Alignment,
{
	inner: Mutex<Stalloc<L, B>>,
}

impl<const L: usize, const B: usize> SyncStalloc<L, B>
where
	Align<B>: Alignment,
{
	pub const fn new() -> Self {
		assert!(L >= 1 && L <= 0xffff, "block count must be in 1..65536");
		Self {
			inner: Mutex::new(Stalloc::<L, B>::new()),
		}
	}

	/// Checks if the allocator is completely out of memory.
	/// If this is false, then you are guaranteed to be able to allocate
	/// a layout with a size and alignment of `B` bytes.
	/// This runs in O(1).
	/// Unlike `Stalloc::is_oom`, this method can return `None` if the Mutex is
	/// poisoned, meaning that we weren't able to acquire a lock.
	pub fn is_oom(&self) -> Option<bool> {
		self.inner.lock().ok().map(|locked| locked.is_oom())
	}

	/// Checks if the allocator is empty.
	/// If this is true, then you are guaranteed to be able to allocate
	/// a layout with a size of `B * L` bytes and an alignment of `B` bytes.
	/// If this is false, then this is guaranteed to be impossible.
	/// This runs in O(1).
	/// Unlike `Stalloc::is_oom`, this method can return `None` if the Mutex is
	/// poisoned, meaning that we weren't able to acquire a lock.
	pub fn is_empty(&self) -> Option<bool> {
		self.inner.lock().ok().map(|locked| locked.is_empty())
	}

	/// # Safety
	///
	/// Calling this function immediately invalidates all pointers into the allocator. Calling
	/// deallocate() with an invalidated pointer may result in the free list being corrupted.
	pub unsafe fn clear(&self) {
		if let Ok(locked) = self.inner.lock() {
			// SAFETY: Upheld by the caller.
			unsafe {
				locked.clear();
			}
		}
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
		let locked = self.inner.lock().map_err(|_| fmt::Error)?;
		write!(f, "{:?}", locked)
	}
}

unsafe impl<const L: usize, const B: usize> GlobalAlloc for SyncStalloc<L, B>
where
	Align<B>: Alignment,
{
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		self.inner
			.lock()
			.ok()
			.and_then(|locked_stalloc| locked_stalloc.allocate(layout).ok())
			.map(|p| p.as_ptr().cast())
			.unwrap_or(ptr::null_mut())
	}

	unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
		self.inner
			.lock()
			.ok()
			.and_then(|locked_inner| locked_inner.allocate_zeroed(layout).ok())
			.map(|p| p.as_ptr().cast())
			.unwrap_or(ptr::null_mut())
	}

	unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
		// SAFETY: upheld by the caller.
		self.inner.lock().ok().inspect(|locked_inner| unsafe {
			locked_inner.deallocate(NonNull::new_unchecked(ptr), layout)
		});
	}

	unsafe fn realloc(&self, ptr: *mut u8, old_layout: Layout, new_size: usize) -> *mut u8 {
		// SAFETY: upheld by the caller.
		let new_layout = unsafe { Layout::from_size_align_unchecked(new_size, old_layout.align()) };

		if new_size > old_layout.size() {
			// SAFETY: upheld by the caller.
			unsafe {
				let nonnull = NonNull::new_unchecked(ptr);
				self.inner
					.lock()
					.ok()
					.and_then(|locked| locked.grow(nonnull, old_layout, new_layout).ok())
					.map(|p| p.as_ptr().cast())
					.unwrap_or(ptr::null_mut())
			}
		} else {
			// SAFETY: upheld by the caller.
			// Note: if `new_size` == `old_layout.size()`, this should be a no-op.
			unsafe {
				let nonnull = NonNull::new_unchecked(ptr);
				self.inner
					.lock()
					.ok()
					.and_then(|locked| locked.shrink(nonnull, old_layout, new_layout).ok())
					.map(|p| p.as_ptr().cast())
					.unwrap_or(ptr::null_mut())
			}
		}
	}
}

unsafe impl<const L: usize, const B: usize> Allocator for SyncStalloc<L, B>
where
	Align<B>: Alignment,
{
	fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
		self.inner
			.lock()
			.map(|locked_stalloc| locked_stalloc.allocate(layout))
			.unwrap_or(Err(AllocError))
	}

	unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
		// SAFETY: Upheld by the caller.
		unsafe {
			if let Ok(locked_stalloc) = self.inner.lock() {
				locked_stalloc.deallocate(ptr, layout);
			}
		}
	}

	fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
		self.inner
			.lock()
			.map(|locked_stalloc| locked_stalloc.allocate(layout))
			.unwrap_or(Err(AllocError))
	}

	unsafe fn grow(
		&self,
		ptr: NonNull<u8>,
		old_layout: Layout,
		new_layout: Layout,
	) -> Result<NonNull<[u8]>, std::alloc::AllocError> {
		// SAFETY: Upheld by the caller.
		unsafe {
			self.inner
				.lock()
				.map(|locked_stalloc| locked_stalloc.grow(ptr, old_layout, new_layout))
				.unwrap_or(Err(AllocError))
		}
	}

	unsafe fn grow_zeroed(
		&self,
		ptr: NonNull<u8>,
		old_layout: Layout,
		new_layout: Layout,
	) -> Result<NonNull<[u8]>, std::alloc::AllocError> {
		// SAFETY: Upheld by the caller.
		unsafe {
			self.inner
				.lock()
				.map(|locked_stalloc| locked_stalloc.grow_zeroed(ptr, old_layout, new_layout))
				.unwrap_or(Err(AllocError))
		}
	}

	unsafe fn shrink(
		&self,
		ptr: NonNull<u8>,
		old_layout: Layout,
		new_layout: Layout,
	) -> Result<NonNull<[u8]>, std::alloc::AllocError> {
		// SAFETY: Upheld by the caller.
		unsafe {
			self.inner
				.lock()
				.map(|locked_stalloc| locked_stalloc.shrink(ptr, old_layout, new_layout))
				.unwrap_or(Err(AllocError))
		}
	}

	fn by_ref(&self) -> &Self
	where
		Self: Sized,
	{
		self
	}
}
