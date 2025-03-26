#![feature(allocator_api)]
use stalloc::Stalloc;
use std::{mem, time::Instant};

fn main() {
	let start = Instant::now();
	for _ in 0..10_000_000 {
		let alloc = Stalloc::<200, 4>::new();

		let mut a = Vec::new_in(&alloc);
		let mut b = Vec::new_in(&alloc);
		for i in 0..10 {
			a.push(i);
			b.push(i);
		}

		// Since the allocator is about to get dropped anyway, no need to drop the individual vectors.
		mem::forget(a);
		mem::forget(b);
	}

	println!("{}", start.elapsed().as_millis());
}
