use stalloc::{AllocChain, SyncStalloc};

use std::{alloc::System, hint::black_box, time::Instant};

// Create a global allocator with 1024 blocks of stack memory,
// but fall back to the system allocator if we ever OOM.
// Note: changing this to `UnsafeStalloc` almost doubles speed...
#[global_allocator]
static GLOBAL: AllocChain<SyncStalloc<1024, 8>, System> = SyncStalloc::new().chain(&System);

fn main() {
	let start = Instant::now();

	let mut big_strings = vec![];

	// Now create lots of small strings
	for i in 0..100_000_000 {
		black_box(String::from("hello!"));

		// Every once in a while, create and store a really big string
		if i % 10000 == 0 {
			big_strings.push("x".repeat(100_000));
		}
	}

	for s in big_strings {
		black_box(s);
	}

	println!("Elapsed: {}ms", start.elapsed().as_millis());
}
