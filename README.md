Stalloc (Stack + alloc) is a fast first-fit memory allocator written in Rust. From my benchmarking, it can be over 3x as fast as the default OS allocator! This is because all memory is allocated from the stack, which allows it to avoid all OS overhead.

Note that Stalloc uses a fixed amount of memory. If it ever runs out, it could result in your program crashing immediately. Stalloc is especially good for programs that make lots of small allocations. Stalloc is also extremely memory-efficient — you can create a "heap" containing less than a dozen bytes!

When you create a Stallocator, you configure it with two numbers: `L` is the number of blocks, and `B` is the size of each block in bytes. The total size of this type comes out to `L * B + 4` bytes, of which `L * B` can be used (4 bytes are needed to hold some metadata).

There are three main ways to use this library:

## With the allocator API
```rs
#![feature(allocator_api)]

let alloc = Stalloc::<200, 4>::new(); // 200 blocks, 4 bytes each
let mut v = Vec::new_in(&alloc);
v.push(25);

// Since the allocator is about to get dropped anyway, no need to call the destructor of `v`.
mem::forget(v);
// `alloc` gets dropped at the end of the scope
```

## With the unsafe APIs
```rs
let alloc = Stalloc::<80, 8>::new();

let alignment = 1; // measured in block size, so 8 bytes
let ptr = unsafe { alloc.allocate_blocks(80, alignment) }.expect("allocation failed");
assert!(alloc.is_oom());
// do stuff with your new allocation

// later...
unsafe {
	alloc.deallocate_blocks(ptr, alignment);
}
```

## As a global allocator
```rs
#[global_allocator]
static GLOBAL: SyncStalloc<1000, 4> = SyncStalloc::new();

fn main() {
	// make some allocations and stuff
	let v = vec![1, 2, 3, 4, 5];
}
```

If your program is single-threaded, you can avoid a little bit of overhead by using `UnsafeStalloc`.
```rs
#[global_allocator]
static GLOBAL: UnsafeStalloc<1000, 4> = unsafe { UnsafeStalloc::new() };
```

See the `examples` folder for a full program using Stalloc.

You can quickly try out the library by running:
```
git clone https://github.com/abgros/stalloc.git
cd stalloc
cargo test
```
Then run the examples with:
```
cargo run --example "name of the example" --release
```