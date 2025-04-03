use core::alloc::{GlobalAlloc, Layout};

/// A trait representing an allocator that another allocator can be chained to.
///
/// # Safety
/// `addr_in_bounds` must return true if and only if the address could belong to
/// a pointer which is valid for the allocator. This trait is used to decide
/// which allocator to call when the user calls `deallocate()` and related functions.
pub unsafe trait ChainableAlloc {
	/// Checks whether a certain address is contained within the allocator. This
	/// is called when using `deallocate()` and related functions in order to
	/// determine which allocator needs to free the pointer.
	fn addr_in_bounds(&self, addr: usize) -> bool;
}

/// A chain of allocators. If the first allocator is exhuasted, the second one is used as a fallback.
///
/// # Examples
/// ```
/// // If the `SyncStalloc` is full, fall back to the system allocator.
/// use stalloc::{SyncStalloc, Stalloc};
/// use std::alloc::System;
///
/// let alloc_with_fallback = SyncStalloc::<1024, 8>::new().chain(&System);
///
/// let crazy_chain = Stalloc::<128, 4>::new()
///     .chain(&Stalloc::<1024, 8>::new())
///     .chain(&Stalloc::<8192, 16>::new())
///     .chain(&System);
/// ```
pub struct AllocChain<'a, A, B>(A, &'a B);

impl<'a, A, B> AllocChain<'a, A, B> {
	/// Initializes a new `AllocChain`.
	pub const fn new(a: A, b: &'a B) -> Self {
		Self(a, b)
	}

	/// Creates a new `AllocChain` containing this chain and `next`.
	pub const fn chain<T>(self, next: &T) -> AllocChain<'_, Self, T>
	where
		Self: Sized,
	{
		AllocChain::new(self, next)
	}
}

unsafe impl<A: GlobalAlloc + ChainableAlloc, B: GlobalAlloc> GlobalAlloc for AllocChain<'_, A, B> {
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		let ptr_a = unsafe { self.0.alloc(layout) };
		if ptr_a.is_null() {
			unsafe { self.1.alloc(layout) }
		} else {
			ptr_a
		}
	}

	unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
		if self.0.addr_in_bounds(ptr.addr()) {
			unsafe { self.0.dealloc(ptr, layout) };
		} else {
			unsafe { self.1.dealloc(ptr, layout) };
		}
	}

	unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
		if self.0.addr_in_bounds(ptr.addr()) {
			let ptr_a = unsafe { self.0.realloc(ptr, layout, new_size) };
			if !ptr_a.is_null() {
				return ptr_a;
			}

			let layout_b = unsafe { Layout::from_size_align_unchecked(new_size, layout.align()) };
			let ptr_b = unsafe { self.1.alloc(layout_b) };

			if !ptr_b.is_null() {
				// Copy the allocation from `A` to `B`.
				unsafe {
					ptr.copy_to_nonoverlapping(ptr_b, layout.size());
					self.0.dealloc(ptr, layout);
				}
			}

			// This is either a valid pointer or null.
			ptr_b
		} else {
			unsafe { self.1.realloc(ptr, layout, new_size) }
			// Don't fall back to `A`.
		}
	}
}

#[cfg(feature = "allocator-api")]
use core::{
	alloc::{AllocError, Allocator},
	ptr::NonNull,
};

#[cfg(feature = "allocator-api")]
unsafe impl<A: Allocator + ChainableAlloc, B: Allocator> Allocator for AllocChain<'_, A, B> {
	fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
		self.0.allocate(layout).or_else(|_| self.1.allocate(layout))
	}

	unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
		if self.0.addr_in_bounds(ptr.addr().into()) {
			unsafe { self.0.deallocate(ptr, layout) };
		} else {
			unsafe { self.1.deallocate(ptr, layout) }
		}
	}

	unsafe fn grow(
		&self,
		ptr: NonNull<u8>,
		old_layout: Layout,
		new_layout: Layout,
	) -> Result<NonNull<[u8]>, AllocError> {
		if self.0.addr_in_bounds(ptr.addr().into()) {
			let res_a = unsafe { self.0.grow(ptr, old_layout, new_layout) };
			if res_a.is_ok() {
				return res_a;
			}

			let res_b = self.1.allocate(new_layout);
			if let Ok(ptr_b) = res_b {
				// Copy the allocation from `A` to `B`.
				unsafe {
					ptr.copy_to_nonoverlapping(ptr_b.cast(), old_layout.size());
					self.0.deallocate(ptr, old_layout);
				}
			}

			res_b
		} else {
			unsafe { self.1.grow(ptr, old_layout, new_layout) }
			// Don't fall back to `A`.
		}
	}

	unsafe fn grow_zeroed(
		&self,
		ptr: NonNull<u8>,
		old_layout: Layout,
		new_layout: Layout,
	) -> Result<NonNull<[u8]>, AllocError> {
		unsafe {
			// SAFETY: Upheld by the caller.
			let new_ptr = self.grow(ptr, old_layout, new_layout)?;
			let count = new_ptr.len() - old_layout.size();

			// SAFETY: We are filling in the extra capacity with zeros.
			new_ptr
				.cast::<u8>()
				.add(old_layout.size())
				.write_bytes(0, count);

			Ok(new_ptr)
		}
	}

	unsafe fn shrink(
		&self,
		ptr: NonNull<u8>,
		old_layout: Layout,
		new_layout: Layout,
	) -> Result<core::ptr::NonNull<[u8]>, core::alloc::AllocError> {
		if self.0.addr_in_bounds(ptr.addr().into()) {
			let res_a = unsafe { self.0.shrink(ptr, old_layout, new_layout) };
			if res_a.is_ok() {
				return res_a;
			}

			let res_b = self.1.allocate(new_layout);
			if let Ok(ptr_b) = res_b {
				// Copy the allocation from `A` to `B`.
				unsafe {
					ptr.copy_to_nonoverlapping(ptr_b.cast(), old_layout.size());
					self.0.deallocate(ptr, old_layout);
				}
			}

			res_b
		} else {
			unsafe { self.1.shrink(ptr, old_layout, new_layout) }
			// Don't fall back to `A`.
		}
	}

	fn by_ref(&self) -> &Self
	where
		Self: Sized,
	{
		self
	}
}
