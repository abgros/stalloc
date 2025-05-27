Stalloc (Stack + alloc) is a fast first-fit memory allocator. From my benchmarking, it can be over 3x as fast as the default OS allocator! This is because all memory is allocated from the stack, which allows it to avoid all OS overhead. Since it doesn't rely on the OS (aside from `SyncStalloc`), this library is `no_std` compatible.

Note that Stalloc uses a fixed amount of memory. If it ever runs out, it could result in your program crashing immediately. Stalloc is especially good for programs that make lots of small allocations.

Stalloc is extremely memory-efficient. Within a 32-byte "heap", you can allocate eight `Box<u32>`s, free them, then allocate four `Box<u64>`s, free them, and then allocate two `Box<u128>`s. This can be especially useful if you're working in a very memory-constrained environment and you need a static upper limit on your application's memory usage.

There are three main ways to use this library:

## With the allocator API (requires the `allocator-api` feature to be enabled in your Cargo.toml)
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
let ptr = unsafe { alloc.allocate_blocks(80, alignment) }.unwrap();
assert!(alloc.is_oom());
// do stuff with your new allocation

// later...
unsafe {
	alloc.deallocate_blocks(ptr, 80);
}
```

## As a global allocator
```rs
#[global_allocator]
static GLOBAL: SyncStalloc<1000, 4> = SyncStalloc::new();

fn main() {
	// allocations and stuff
	let v = vec![1, 2, 3, 4, 5];

	// we can check on the allocator state
	println!("{GLOBAL:?}");
}
```

If your program is single-threaded, you can avoid a little bit of overhead by using `UnsafeStalloc`, which isn't thread-safe.
```rs
#[global_allocator]
static GLOBAL: UnsafeStalloc<1000, 4> = unsafe { UnsafeStalloc::new() };
```

To avoid the risk of OOM, you can create an allocator chain, which uses the next one as a fallback if something has gone wrong:
```rs
// Create an allocator chain, where we try to use the fast `SyncStalloc`, but fall back to `System`.
#[global_allocator]
static GLOBAL: AllocChain<SyncStalloc<1000, 8>, System> = SyncStalloc::new().chain(&System);
```

When you create a Stallocator, you configure it with two numbers: `L` is the number of blocks, and `B` is the size of each block in bytes. The total size of this type comes out to `L * B + 4` bytes, of which `L * B` can be used (4 bytes are needed to hold some metadata). The buffer is automatically aligned to `B`. If you want it to be more aligned than that, you can create a wrapper like this:

```rs
#[repr(align(16))] // aligned to 16 bytes
struct MoreAlignedStalloc(Stalloc<8, 4>); // eight blocks of four bytes each
```

To use this library with no-std, add the following to your `Cargo.toml`:
```
[dependencies]
stalloc = {version = <latest>, default-features = false}
```
To use this library with the allocator API, add the following instead:
```
[dependencies]
stalloc = {version = <latest>, features = ["allocator-api"]}
```
Or, if you prefer using the [allocator-api2](https://crates.io/crates/allocator-api2) crate with the stable compiler:
```
[dependencies]
stalloc = {version = <latest>, features = ["allocator-api2"]}
```