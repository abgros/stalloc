use core::alloc::{GlobalAlloc, Layout};
use core::fmt::{self, Debug, Formatter};
use core::hint::assert_unchecked;
use core::ops::Deref;
use core::ptr::{self, NonNull};

use crate::align::{Align, Alignment};
use crate::{AllocChain, ChainableAlloc, Stalloc};

/// A wrapper around `Stalloc` that implements both `Sync` and `GlobalAlloc`.
///
/// This type is unsafe to create, because it does not prevent data races.
/// Therefore, it is encouraged to only use it in single-threaded environments.
#[repr(transparent)]
pub struct UnsafeStalloc<const L: usize, const B: usize>(Stalloc<L, B>)
where
	Align<B>: Alignment;

impl<const L: usize, const B: usize> Deref for UnsafeStalloc<L, B>
where
	Align<B>: Alignment,
{
	type Target = Stalloc<L, B>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<const L: usize, const B: usize> Debug for UnsafeStalloc<L, B>
where
	Align<B>: Alignment,
{
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "{:?}", self.0)
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
	///
	/// # Examples
	/// ```
	/// use stalloc::UnsafeStalloc;
	///
	/// let alloc = unsafe { UnsafeStalloc::<200, 8>::new() };
	/// ```
	#[must_use]
	pub const unsafe fn new() -> Self {
		Self(Stalloc::<L, B>::new())
	}
}

unsafe impl<const L: usize, const B: usize> Sync for UnsafeStalloc<L, B> where Align<B>: Alignment {}

#[cfg(feature = "allocator-api")]
use core::alloc::{AllocError, Allocator};

#[cfg(feature = "allocator-api")]
unsafe impl<const L: usize, const B: usize> Allocator for &UnsafeStalloc<L, B>
where
	Align<B>: Alignment,
{
	fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
		(&self.0).allocate(layout)
	}

	unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
		// SAFETY: Upheld by the caller.
		unsafe {
			(&self.0).deallocate(ptr, layout);
		}
	}

	fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
		(&self.0).allocate_zeroed(layout)
	}

	unsafe fn grow(
		&self,
		ptr: NonNull<u8>,
		old_layout: Layout,
		new_layout: Layout,
	) -> Result<NonNull<[u8]>, AllocError> {
		// SAFETY: Upheld by the caller.
		unsafe { (&self.0).grow(ptr, old_layout, new_layout) }
	}

	unsafe fn grow_zeroed(
		&self,
		ptr: NonNull<u8>,
		old_layout: Layout,
		new_layout: Layout,
	) -> Result<NonNull<[u8]>, AllocError> {
		// SAFETY: Upheld by the caller.
		unsafe { (&self.0).grow_zeroed(ptr, old_layout, new_layout) }
	}

	unsafe fn shrink(
		&self,
		ptr: NonNull<u8>,
		old_layout: Layout,
		new_layout: Layout,
	) -> Result<NonNull<[u8]>, AllocError> {
		// SAFETY: Upheld by the caller.
		unsafe { (&self.0).shrink(ptr, old_layout, new_layout) }
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
			self.allocate_blocks(size, align)
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
		let size = layout.size().div_ceil(B);

		// SAFETY: Upheld by the caller.
		unsafe {
			self.deallocate_blocks(NonNull::new_unchecked(ptr), size);
		}
	}

	unsafe fn realloc(&self, ptr: *mut u8, old_layout: Layout, new_size: usize) -> *mut u8 {
		// Assert unsafe precondition.
		unsafe {
			assert_unchecked(new_size > 0);
		}

		let old_size = old_layout.size() / B;
		let new_size = new_size.div_ceil(B);

		unsafe {
			// SAFETY: Upheld by the caller.
			let ptr: NonNull<u8> = NonNull::new_unchecked(ptr);

			// SAFETY: Upheld by the caller.
			if new_size > old_size && self.grow_in_place(ptr, old_size, new_size).is_ok() {
				return ptr.as_ptr();
			} else if new_size > old_size {
				// Reallocate and copy.
				// SAFETY: We have made sure that `new_size > 0` and that `align` is valid.
				let Ok(new) = self.allocate_blocks(new_size, old_layout.align()) else {
					return ptr::null_mut();
				};

				// SAFETY: We are copying all the necessary bytes from `ptr` into `new`.
				// `ptr` and `new` both point to an allocation of at least `old_layout.size()` bytes.
				ptr::copy_nonoverlapping(ptr.as_ptr(), new.as_ptr(), old_layout.size());

				// SAFETY: The caller upholds that old_size > 0.
				self.deallocate_blocks(ptr, old_size);

				return new.as_ptr();
			} else if old_size > new_size {
				// SAFETY: Upheld by the caller.
				self.shrink_in_place(ptr, old_size, new_size);
			}

			ptr.as_ptr()
		}
	}
}

unsafe impl<const L: usize, const B: usize> ChainableAlloc for UnsafeStalloc<L, B>
where
	Align<B>: Alignment,
{
	fn addr_in_bounds(&self, addr: usize) -> bool {
		self.0.addr_in_bounds(addr)
	}
}

impl<const L: usize, const B: usize> UnsafeStalloc<L, B>
where
	Align<B>: Alignment,
{
	/// Creates a new `AllocChain` containing this allocator and `next`.
	pub const fn chain<T>(self, next: &T) -> AllocChain<'_, Self, T>
	where
		Self: Sized,
	{
		AllocChain::new(self, next)
	}
}
