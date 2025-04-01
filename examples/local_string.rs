use stalloc::Stalloc;
use std::{hint::black_box, mem, ptr::NonNull, time::Instant};

fn main() {
	let start = Instant::now();

	const BLOCK_SIZE: usize = 4;
	let s = Stalloc::<200, BLOCK_SIZE>::new();

	for _ in 0..100_000_000 {
		// SAFETY: `msg` will never try to deallocate or reallocate.
		let mut msg = unsafe {
			String::from_raw_parts(
				s.allocate_blocks(50, 1).unwrap().as_ptr(),
				0,
				50 * BLOCK_SIZE,
			)
		};
		msg.push_str("Hello, ");
		msg.push_str("world!");
		msg = black_box(msg);

		unsafe {
			s.deallocate_blocks(NonNull::new_unchecked(msg.as_mut_ptr()), 50);
		}

		// If we let `msg` drop itself, it will call `dealloc()` on the global allocator (not `s`),
		// resulting in undefined behaviour.
		mem::forget(msg);
	}

	println!("Elapsed: {}ms", start.elapsed().as_millis());
}
