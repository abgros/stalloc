Stalloc (Stack + alloc) is a fast first-fit memory allocator. From my benchmarking, it can be 3x as fast as the default OS allocator! This is because all memory is allocated from the stack, which allows it to avoid all OS overhead.

Note that Stalloc has a fixed amount of memory. If it doesn't have any more, it could result in your program crashing immediately.

There are two main ways to use this library:

## With the allocator API
```rs
#![feature(allocator_api)]

let alloc = Stalloc::<200, 4>::new();
let mut v = Vec::new_in(&alloc);
v.push(25);

// Since the allocator is about to get dropped anyway, no need to call the destructor of `v`.
mem::forget(v);
// `alloc` gets dropped at the end of the scope
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

See the `examples` folder for a full program using `Stalloc`.