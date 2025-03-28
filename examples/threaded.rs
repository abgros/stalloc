#![feature(allocator_api)]

use std::{hint::black_box, thread, time::Instant};

use stalloc::SyncStalloc;

const THREAD_COUNT: usize = 6;

fn main() {
	let start = Instant::now();

	for _ in 0..5000 {
		let alloc = SyncStalloc::<THREAD_COUNT, 4>::new();

		thread::scope(|s| {
			for _ in 0..THREAD_COUNT {
				s.spawn(|| {
					for _ in 0..1000 {
						black_box(Box::new_in(3, &alloc));
					}
				});
			}
		});
	}

	println!("{}", start.elapsed().as_millis());
}
