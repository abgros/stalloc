use stalloc::UnsafeStalloc;
use std::{mem, time::Instant};

// Create a global allocator with 1000 blocks, each 4 bytes in length.
// SAFETY: The program is single-threaded.
#[global_allocator]
static GLOBAL: UnsafeStalloc<1000, 4> = unsafe { UnsafeStalloc::new() };

fn main() {
	let start = Instant::now();
	for _ in 0..10_000_000 {
		let mut a = vec![];
		let mut b = vec![];
		for i in 0..10 {
			a.push(i);
			b.push(i);
		}

		mem::forget(a);
		mem::forget(b);

		// By clearing the global allocator, we can quickly drop both vectors together.
		// SAFETY: There are no more active allocations into `GLOBAL`.
		unsafe {
			GLOBAL.clear();
		}
	}

	println!("Elapsed: {}ms", start.elapsed().as_millis());
}
