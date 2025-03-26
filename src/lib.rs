#![feature(allocator_api)]

use core::alloc::{AllocError, Allocator, Layout};
use core::cell::UnsafeCell;
use core::fmt::{self, Debug, Formatter};
use core::mem::MaybeUninit;
use core::ptr::{self, NonNull};

mod align;
use align::*;
mod unsafestalloc;
pub use unsafestalloc::*;
mod syncstalloc;
pub use syncstalloc::*;

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Default)]
struct Header {
	next: u16,
	length: u16,
}

#[derive(Clone, Copy)]
union Block<const B: usize>
where
	Align<B>: Alignment,
{
	header: Header,
	bytes: [MaybeUninit<u8>; B],
	_align: Align<B>,
}

// The `base` Header has a unique meaning here. Because `base.length` is useless (always 0),
// we use it as a special flag to check whether `data` is completely filled. Every call to
// `allocate()` and related functions must verify that base.length != OOM_MARKER.
const OOM_MARKER: u16 = u16::MAX;

/// A fast first-fit memory allocator.
///
/// When you create an instance of this allocator, you pass in a value for `L` and `B`.
/// `L` is the number of blocks, and `B` is the size of each block in bytes. The total size of this type
/// comes out to `L * B + 4` bytes, of which `L * B` can be used (4 bytes are needed to hold some metadata).
/// `B` must be a power of two from 4 and 2^29, and `L` must be a number in the range `1..65536`.
///
/// `B` represents the smallest unit of memory that the allocator can manage. If `B == 16`, then asking
/// for 17 bytes will give you a 32 byte allocation (the amount is rounded up).
/// The alignment of the allocator is always equal to `B`. For maximum efficiency, it is recommended
/// to set `B` equal to the alignment of the type you expect to store the most of. For example, if you're storing
/// a lot of `u64`s, you should set `B == 8`.
pub struct Stalloc<const L: usize, const B: usize>
where
	Align<B>: Alignment,
{
	base: UnsafeCell<Header>,
	data: UnsafeCell<[Block<B>; L]>,
}

impl<const L: usize, const B: usize> Stalloc<L, B>
where
	Align<B>: Alignment,
{
	pub const fn new() -> Self {
		assert!(L >= 1 && L <= 0xffff, "block count must be in 1..65536");
		assert!(B >= 4, "block size must be at least 4 bytes");

		let mut blocks = [Block {
			bytes: [MaybeUninit::uninit(); B],
		}; L];

		// Write the first header.
		blocks[0].header = Header {
			next: 0,
			length: L as u16,
		};

		Stalloc {
			base: UnsafeCell::new(Header { next: 0, length: 0 }),
			data: UnsafeCell::new(blocks),
		}
	}

	/// Checks if the allocator is completely out of memory.
	/// If this is false, then you are guaranteed to be able to allocate
	/// a layout with a size and alignment of `B` bytes.
	/// This runs in O(1).
	pub fn is_oom(&self) -> bool {
		unsafe { *self.base.get() }.length == OOM_MARKER
	}

	/// Checks if the allocator is empty.
	/// If this is true, then you are guaranteed to be able to allocate
	/// a layout with a size of `B * L` bytes and an alignment of `B` bytes.
	/// If this is false, then this is guaranteed to be impossible.
	/// This runs in O(1).
	pub fn is_empty(&self) -> bool {
		!self.is_oom() && unsafe { *self.base.get() }.next == 0
	}

	/// # Safety
	///
	/// Calling this function immediately invalidates all pointers into the allocator. Calling
	/// deallocate() with an invalidated pointer may result in the free list being corrupted.
	pub unsafe fn clear(&self) {
		unsafe {
			(*self.base.get()).next = 0;
			(*self.base.get()).length = 0;
			(*self.header_at(0)).next = 0;
			(*self.header_at(0)).length = L as u16;
		}
	}

	/// Get the index of a pointer to `data`. This function is always safe
	/// to call, but the result may not be meaningful.
	/// Even if the header is not at the start of the block (compiler's choice),
	/// dividing by B rounds down and produces the correct result.
	fn index_of(&self, ptr: *mut Header) -> usize {
		(ptr.addr() - self.data.get().addr()) / B
	}

	/// Safety precondition: idx must be in `0..L`.
	unsafe fn block_at(&self, idx: usize) -> *mut Block<B> {
		let root: *mut Block<B> = self.data.get().cast();
		unsafe { root.add(idx) }
	}

	/// Safety precondition: idx must be in `0..L`.
	unsafe fn header_at(&self, idx: usize) -> *mut Header {
		unsafe { &raw mut (*self.block_at(idx)).header }
	}

	/// This function doesn't dereference `ptr`, so it is always safe to call.
	fn nonnull_to_header(&self, ptr: NonNull<u8>) -> *mut Header {
		unsafe { &raw mut (*ptr.as_ptr().cast::<Block<B>>()).header }
	}

	/// This function always is safe to call. If `idx` is very large,
	/// the returned value will simply be the last header in the free list.
	/// Note: this function may return a pointer to `base`.
	fn header_before(&self, idx: usize) -> *mut Header {
		let mut ptr = self.base.get();

		unsafe {
			if (*ptr).length == OOM_MARKER || (*ptr).next as usize >= idx {
				return ptr;
			}

			loop {
				ptr = self.header_at((*ptr).next as usize);
				let next_idx = (*ptr).next as usize;
				if next_idx == 0 || next_idx >= idx {
					return ptr;
				}
			}
		}
	}
}

impl<const L: usize, const B: usize> Debug for Stalloc<L, B>
where
	Align<B>: Alignment,
{
	fn fmt(&self, f: &mut Formatter) -> fmt::Result {
		write!(f, "Stallocator with {L} blocks of {B} bytes each")?;

		let mut ptr = self.base.get();
		if unsafe { (*ptr).length } == OOM_MARKER {
			return write!(f, "\n\tNo free blocks (OOM)");
		}

		loop {
			unsafe {
				let idx = (*ptr).next as usize;
				ptr = self.header_at(idx);

				let length = (*ptr).length as usize;
				if length == 1 {
					write!(f, "\n\tindex {idx}: {length} free block")?;
				} else {
					write!(f, "\n\tindex {idx}: {length} free blocks")?;
				}

				if (*ptr).next == 0 {
					return Ok(());
				}
			}
		}
	}
}

impl<const L: usize, const B: usize> Default for Stalloc<L, B>
where
	Align<B>: Alignment,
{
	fn default() -> Self {
		Self::new()
	}
}

unsafe impl<const L: usize, const B: usize> Allocator for Stalloc<L, B>
where
	Align<B>: Alignment,
{
	fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
		// We can only allocate memory in units of `B`, so round up.
		let size = layout.size().div_ceil(B);
		let align = layout.align().div_ceil(B);

		// If `size` is zero, give away some random pointer since it can't be used anyway.
		if size == 0 {
			let dangling = NonNull::new(layout.align() as _).unwrap();
			return Ok(NonNull::slice_from_raw_parts(dangling, 0));
		}

		// Check whether `data` is completely full, or the requested allocation is obviously too large.
		if self.is_oom() || size > L || align > L {
			return Err(AllocError);
		}

		// Loop through the free list, and find the first header whose length satisfies the layout.
		unsafe {
			// `prev` and `curr` are pointers that run through the free list.
			let base = self.base.get();
			let mut prev = base;
			let mut curr = self.header_at((*base).next as usize);

			loop {
				let curr_idx = (*prev).next as usize;
				let next_idx = (*curr).next as usize;

				// Check if the current free chunk satisfies the layout.
				let curr_chunk_len = (*curr).length as usize;

				// If the alignment is more than 1, there might be spare blocks in front.
				// If it is extremely large, there might have to be more spare blocks than are available.
				let spare_front: usize = (curr.addr() / B).wrapping_neg() % align;

				if spare_front + size <= curr_chunk_len {
					let avail_blocks = curr_chunk_len - spare_front;
					let avail_blocks_ptr = self.block_at(curr_idx + spare_front);
					let spare_back = avail_blocks - size;

					// If there are spare blocks, add them to the free list.
					if spare_back > 0 {
						let spare_back_idx = curr_idx + spare_front + size;
						let spare_back_ptr = self.header_at(spare_back_idx);
						(*spare_back_ptr).next = next_idx as u16;
						(*spare_back_ptr).length = spare_back as u16;

						if spare_front > 0 {
							(*curr).next = spare_back_idx as u16;
							(*curr).length = spare_front as u16;
						} else {
							(*prev).next = spare_back_idx as u16;
						}
					} else if spare_front > 0 {
						(*curr).next = (curr_idx + spare_front + size) as u16;
						(*curr).length = spare_front as u16;
						(*prev).next = next_idx as u16;
					} else {
						(*prev).next = next_idx as u16;
						// If this is the last block of memory, set the OOM marker.
						if next_idx == 0 {
							(*base).length = OOM_MARKER;
						}
					}

					return Ok(NonNull::slice_from_raw_parts(
						NonNull::new_unchecked(avail_blocks_ptr.cast()),
						size * B,
					));
				}

				// Check if we've already made a whole loop around without finding anything.
				if next_idx == 0 {
					return Err(AllocError);
				}

				prev = curr;
				curr = self.header_at(next_idx);
			}
		}
	}

	unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
		// If the size is 0, the pointer is dangling, so do nothing.
		if layout.size() == 0 {
			return;
		}

		let size = layout.size().div_ceil(B);
		let freed_ptr = self.nonnull_to_header(ptr);
		let freed_idx = self.index_of(freed_ptr);
		let base = self.base.get();
		let before = self.header_before(freed_idx);

		unsafe {
			let prev_next = (*before).next as usize;
			(*freed_ptr).next = prev_next as u16;
			(*freed_ptr).length = size as u16;

			// Try to merge with the next free block.
			if freed_idx + size == prev_next {
				let header_to_merge = self.header_at(prev_next);
				(*freed_ptr).next = (*header_to_merge).next;
				(*freed_ptr).length += (*header_to_merge).length;
			}

			// Try to merge with the previous free block.
			if before.eq(&base) {
				(*base).next = freed_idx as u16;
				(*base).length = 0;
			} else if self.index_of(before) + (*before).length as usize == freed_idx {
				(*before).next = (*freed_ptr).next;
				(*before).length += (*freed_ptr).length;
			} else {
				// No merge is possible.
				(*before).next = freed_idx as u16;
			}
		}
	}

	unsafe fn grow(
		&self,
		ptr: NonNull<u8>,
		old_layout: Layout,
		new_layout: Layout,
	) -> Result<NonNull<[u8]>, AllocError> {
		let old_size = old_layout.size().div_ceil(B);
		let new_size = new_layout.size().div_ceil(B);

		// If the size hasn't changed, do nothing.
		let needed_chunks = new_size - old_size;
		if needed_chunks == 0 {
			return Ok(NonNull::slice_from_raw_parts(ptr, new_layout.size()));
		}

		// If the old size was 0, the pointer was dangling, so just allocate.
		if old_size == 0 {
			return self.allocate(new_layout);
		}

		let curr_block: *mut Block<B> = ptr.as_ptr().cast();
		let curr_idx = (curr_block.addr() - self.data.get().addr()) / B;
		let prev_free_chunk = self.header_before(curr_idx);

		unsafe {
			// Check if there's room to grow.
			// Note: `next_idx` must be directly after the current allocation.
			// Also, the requested amount of chunks must be within the next free chunk.
			let next_free_idx = (*prev_free_chunk).next as usize;
			let next_free_chunk = self.header_at(next_free_idx);
			let room_to_grow = (*next_free_chunk).length as usize;

			if curr_idx + old_size == next_free_idx && needed_chunks <= room_to_grow {
				let chunk_left_over = room_to_grow - needed_chunks;
				if chunk_left_over > 0 {
					let new_chunk_idx = next_free_idx + needed_chunks;
					let new_chunk_head = self.header_at(new_chunk_idx);

					// Insert the new chunk into the free list.
					(*prev_free_chunk).next = new_chunk_idx as u16;
					(*new_chunk_head).next = (*next_free_chunk).next;
					(*new_chunk_head).length = chunk_left_over as u16;
				} else {
					// The free chunk is completely consumed.
					(*prev_free_chunk).next = (*next_free_chunk).next;

					// If `prev_free_chunk` is the base pointer and we just set it to 0, we are OOM.
					let base = self.base.get();
					if prev_free_chunk.eq(&base) && (*next_free_chunk).next == 0 {
						(*base).length = OOM_MARKER;
					}
				}

				return Ok(NonNull::slice_from_raw_parts(ptr, new_layout.size()));
			}

			// Otherwise just reallocate and copy.
			let new = self.allocate(new_layout)?;
			ptr::copy_nonoverlapping(ptr.as_ptr(), new.as_ptr() as _, old_layout.size());
			self.deallocate(ptr, old_layout);
			Ok(new)
		}
	}

	unsafe fn grow_zeroed(
		&self,
		ptr: NonNull<u8>,
		old_layout: Layout,
		new_layout: Layout,
	) -> Result<NonNull<[u8]>, AllocError> {
		unsafe {
			let new_ptr = self.grow(ptr, old_layout, new_layout)?;
			let count = new_layout.size() - old_layout.size();
			ptr::write_bytes(ptr.as_ptr().add(old_layout.size()), 0, count);
			Ok(new_ptr)
		}
	}

	unsafe fn shrink(
		&self,
		ptr: NonNull<u8>,
		old_layout: Layout,
		new_layout: Layout,
	) -> Result<NonNull<[u8]>, AllocError> {
		let old_size = old_layout.size().div_ceil(B);
		let new_size = new_layout.size().div_ceil(B);

		// Check if the size is zero, in which case the allocation should just be freed.
		if new_size == 0 {
			unsafe { self.deallocate(ptr, old_layout) };
			let dangling = NonNull::new(new_layout.align() as _).unwrap();
			return Ok(NonNull::slice_from_raw_parts(dangling, 0));
		}

		// We have to reallocate only if the alignment isn't good enough anymore.
		if ptr.as_ptr().addr() % new_layout.align() != 0 {
			let new = self.allocate(new_layout)?;
			unsafe {
				ptr::copy_nonoverlapping(ptr.as_ptr(), new.as_ptr() as _, old_layout.size());
				self.deallocate(ptr, old_layout);
			}
			return Ok(new);
		}

		// Check if the size hasn't changed.
		let spare_blocks = old_size - new_size;
		if spare_blocks == 0 {
			return Ok(NonNull::slice_from_raw_parts(ptr, old_size));
		}

		// Shrink in place.
		// This means that a new chunk will be created in the gap.
		let curr_block: *mut Block<B> = ptr.as_ptr().cast();
		let curr_idx = (curr_block.addr() - self.data.get().addr()) / B;
		let new_idx = curr_idx + new_size;

		unsafe {
			// We are definitely no longer OOM.
			(*self.base.get()).length = 0;

			// Check if we can merge the block with a chunk immediately after.
			let prev_free_chunk = self.header_before(curr_idx);
			let next_free_idx = (*prev_free_chunk).next as usize;
			let new_chunk = &raw mut (*curr_block.add(new_size)).header;

			(*prev_free_chunk).next = new_idx as u16;
			if new_idx + spare_blocks == next_free_idx {
				let next_free_chunk = self.header_at(next_free_idx);
				(*new_chunk).next = (*next_free_chunk).next;
				(*new_chunk).length = spare_blocks as u16 + (*next_free_chunk).length;
			} else {
				(*new_chunk).next = next_free_idx as u16;
				(*new_chunk).length = spare_blocks as u16;
			}
		}

		Ok(NonNull::slice_from_raw_parts(ptr, new_size))
	}
}
