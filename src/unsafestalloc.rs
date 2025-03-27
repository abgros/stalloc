use core::alloc::{AllocError, Allocator, GlobalAlloc, Layout};
use core::fmt::{self, Debug, Formatter};
use core::ops::Deref;
use core::ptr::{self, NonNull};
use std::hint::assert_unchecked;

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

impl<const L: usize, const B: usize> Deref for UnsafeStalloc<L, B>
where
	Align<B>: Alignment,
{
	type Target = Stalloc<L, B>;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
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
		let size = layout.size().div_ceil(B);
		let align = layout.align().div_ceil(B);

		// SAFETY: `size` and `align` are valid.
		unsafe {
			self.inner
				.allocate_blocks(size, align)
				.map(|p| p.as_ptr().cast())
				.unwrap_or(ptr::null_mut())
		}
	}

	unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
		let size = layout.size().div_ceil(B);

		// SAFETY: Upheld by the caller.
		let new = unsafe { self.alloc(layout) };
		if !new.is_null() {
			// SAFETY: `new` points to a valid allocation of `size * B` bytes.
			unsafe { ptr::write_bytes(new, 0, size * B) };
		}
		new
	}

	unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
		// SAFETY: upheld by the caller.
		unsafe {
			self.inner
				.deallocate_blocks(NonNull::new_unchecked(ptr), layout.size() * B);
		}
	}

	unsafe fn realloc(&self, ptr: *mut u8, old_layout: Layout, new_size: usize) -> *mut u8 {
		// Assert unsafe precondition.
		unsafe {
			assert_unchecked(new_size > 0);
		}

		// SAFETY: upheld by the caller.
		let ptr: NonNull<u8> = unsafe { NonNull::new_unchecked(ptr) };
		let old_size = old_layout.size() / B;
		let new_size = new_size.div_ceil(B);

		if new_size > old_size {
			// SAFETY: upheld by the caller.
			return unsafe {
				if self.grow_in_place(ptr, old_size, new_size).is_ok() {
					ptr.as_ptr()
				} else {
					// Otherwise just reallocate and copy.
					// SAFETY: We have made sure that `new_size > 0` and that `align` is valid.
					let Ok(new) = self.allocate_blocks(new_size, old_layout.align()) else {
						return ptr::null_mut();
					};

					// SAFETY: We are copying all the necessary bytes from `ptr` into `new`.
					// `ptr` and `new` both point to an allocation of at least `old_layout.size()` bytes.
					ptr::copy_nonoverlapping(ptr.as_ptr(), new.as_ptr(), old_layout.size());

					// SAFETY: The caller upholds that old_size > 0.
					self.deallocate_blocks(ptr, old_size);

					new.as_ptr()
				}
			};
		} else if old_size > new_size {
			// SAFETY: upheld by the caller.
			unsafe {
				self.inner.shrink_in_place(ptr, old_size, new_size);
			}
		}

		ptr.as_ptr()
	}
}
