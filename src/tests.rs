#![allow(clippy::nursery)]
use crate::Stalloc;

extern crate alloc;
extern crate std;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::mem;
use core::mem::MaybeUninit;
#[allow(unused_imports)]
use std::dbg;

#[test]
fn test_vec() {
	let alloc = Stalloc::<1, 4>::new();
	let mut v: Vec<u8, _> = Vec::with_capacity_in(4, &alloc);
	for i in 0..v.capacity() {
		v.push(42 + i as u8);
	}
}

#[test]
fn test_2_vecs() {
	let alloc = Stalloc::<2, 4>::new();
	let mut v: Vec<u8, _> = Vec::with_capacity_in(4, &alloc);
	for i in 0..v.capacity() {
		v.push(42 + i as u8);
	}
	let mut v: Vec<u8, _> = Vec::with_capacity_in(4, &alloc);
	for i in 0..v.capacity() {
		v.push(42 + i as u8);
	}
}

#[test]
fn test_differently_sized_vecs() {
	let alloc = Stalloc::<28, 4>::new();
	let _v: Vec<u32, _> = Vec::with_capacity_in(1, &alloc);
	let _v: Vec<u32, _> = Vec::with_capacity_in(2, &alloc);
	let _v: Vec<u32, _> = Vec::with_capacity_in(3, &alloc);
	let _v: Vec<u32, _> = Vec::with_capacity_in(4, &alloc);
	let _v: Vec<u32, _> = Vec::with_capacity_in(5, &alloc);
	let _v: Vec<u32, _> = Vec::with_capacity_in(6, &alloc);
	let _v: Vec<u32, _> = Vec::with_capacity_in(7, &alloc);
	assert!(alloc.is_oom());
}

#[test]
#[should_panic(expected = "TryReserveError")]
fn test_oom() {
	let alloc = Stalloc::<3, 4>::new();
	let mut v: Vec<u8, _> = Vec::try_with_capacity_in(8, &alloc).unwrap();
	for i in 0..v.capacity() {
		v.push(42 + i as u8);
	}
	let mut v: Vec<u8, _> = Vec::try_with_capacity_in(8, &alloc).unwrap();
	for i in 0..v.capacity() {
		v.push(42 + i as u8);
	}
}

#[test]
#[should_panic(expected = "TryReserveError")]
fn test_oom2() {
	let alloc = Stalloc::<4, 4>::new();
	let _v: Vec<u32, _> = Vec::try_with_capacity_in(1, &alloc).unwrap();
	let _v: Vec<u32, _> = Vec::try_with_capacity_in(1, &alloc).unwrap();
	let _v: Vec<u32, _> = Vec::try_with_capacity_in(1, &alloc).unwrap();
	let _v: Vec<u32, _> = Vec::try_with_capacity_in(1, &alloc).unwrap();
	let _v: Vec<u32, _> = Vec::try_with_capacity_in(1, &alloc).unwrap();
}

#[test]
#[should_panic(expected = "block count must be in 1..65536")]
fn test_invalid_new1() {
	let _alloc = Stalloc::<0, 4>::new();
}

#[test]
#[should_panic(expected = "block count must be in 1..65536")]
fn test_invalid_new2() {
	let _alloc = Stalloc::<100_000, 4>::new();
}

#[test]
#[should_panic(expected = "block size must be at least 4 bytes")]
fn test_invalid_new3() {
	let _alloc = Stalloc::<2, 2>::new();
}

#[test]
#[should_panic(expected = "block size must be at least 4 bytes")]
fn test_invalid_new4() {
	let _alloc = Stalloc::<2, 1>::new();
}

#[test]
fn test_free() {
	let alloc = Stalloc::<4, 4>::new();
	let v: Vec<u32, _> = Vec::with_capacity_in(4, &alloc);
	drop(v);
	let v: Vec<u32, _> = Vec::with_capacity_in(4, &alloc);
	drop(v);
	let v: Vec<u32, _> = Vec::with_capacity_in(4, &alloc);
	drop(v);
	let v: Vec<u32, _> = Vec::with_capacity_in(4, &alloc);
	drop(v);
	assert!(alloc.is_empty());
}

#[test]
fn test_free_and_realloc() {
	let alloc = Stalloc::<4, 4>::new();
	let v1: Vec<u32, _> = Vec::with_capacity_in(1, &alloc);
	let v2: Vec<u32, _> = Vec::with_capacity_in(1, &alloc);
	let v3: Vec<u32, _> = Vec::with_capacity_in(1, &alloc);
	let v4: Vec<u32, _> = Vec::with_capacity_in(1, &alloc);
	drop(v2);
	drop(v4);
	drop(v1);
	drop(v3);
	let v5: Vec<u32, _> = Vec::with_capacity_in(4, &alloc);
	drop(v5);
	assert!(alloc.is_empty());
}

#[test]
fn test_complex_alloc_and_free() {
	let alloc = Stalloc::<64, 8>::new();

	let v1: Vec<u8, _> = Vec::with_capacity_in(4, &alloc);
	let v2: Vec<u16, _> = Vec::with_capacity_in(8, &alloc);
	let v3: Vec<u32, _> = Vec::with_capacity_in(12, &alloc);
	let v4: Vec<u64, _> = Vec::with_capacity_in(6, &alloc);
	drop(v2);
	let v5: Vec<u8, _> = Vec::with_capacity_in(6, &alloc);
	let v6: Vec<u16, _> = Vec::with_capacity_in(3, &alloc);
	drop(v1);
	let v7: Vec<u32, _> = Vec::with_capacity_in(5, &alloc);
	let v8: Vec<u64, _> = Vec::with_capacity_in(2, &alloc);
	drop(v3);
	let v9: Vec<u8, _> = Vec::with_capacity_in(10, &alloc);
	drop(v4);
	drop(v6);
	let v10: Vec<u16, _> = Vec::with_capacity_in(4, &alloc);
	let v11: Vec<u32, _> = Vec::with_capacity_in(7, &alloc);
	drop(v5);
	let v12: Vec<u64, _> = Vec::with_capacity_in(6, &alloc);
	drop(v7);
	drop(v8);
	let v13: Vec<u8, _> = Vec::with_capacity_in(9, &alloc);
	drop(v9);
	let v14: Vec<u16, _> = Vec::with_capacity_in(5, &alloc);
	drop(v10);
	drop(v11);
	drop(v12);
	drop(v13);
	drop(v14);

	assert!(alloc.is_empty());
}

#[test]
fn test_shrink() {
	let alloc = Stalloc::<6, 4>::new();

	let mut v: Vec<u32, _> = Vec::with_capacity_in(6, &alloc);
	assert!(alloc.is_oom());
	v.shrink_to(5);
	assert!(!alloc.is_oom());
	v.shrink_to(4);
	drop(v);
	assert!(alloc.is_empty());
}

#[test]
fn test_shrink2() {
	let alloc = Stalloc::<6, 4>::new();

	let mut v: Vec<u32, _> = Vec::with_capacity_in(6, &alloc);
	v.shrink_to(0);
	assert!(alloc.is_empty());
}

#[test]
fn test_shrink3() {
	let alloc = Stalloc::<10, 4>::new();

	let mut v1: Vec<u32, _> = Vec::with_capacity_in(8, &alloc);
	v1.shrink_to(6);
	let v2: Vec<u32, _> = Vec::with_capacity_in(4, &alloc);
	assert!(alloc.is_oom());
	v1.shrink_to(1);
	let v3: Vec<u32, _> = Vec::with_capacity_in(5, &alloc);

	drop(v2);
	drop(v1);
	drop(v3);

	assert!(alloc.is_empty());
}

#[test]
fn test_grow() {
	let alloc = Stalloc::<6, 4>::new();

	let mut v: Vec<u32, _> = Vec::with_capacity_in(3, &alloc);
	v.reserve_exact(6);
	assert!(alloc.is_oom());
}

#[test]
fn test_grow_realloc() {
	let alloc = Stalloc::<12, 4>::new();

	let mut v1: Vec<u32, _> = Vec::with_capacity_in(3, &alloc);
	let _v2: Vec<u32, _> = Vec::with_capacity_in(3, &alloc);
	v1.reserve_exact(6);
	let _v3: Vec<u32, _> = Vec::with_capacity_in(3, &alloc);
	assert!(alloc.is_oom());
}

#[test]
fn test_multiple_allocations_and_drops() {
	let alloc = Stalloc::<16, 4>::new();

	let mut v1: Vec<u32, _> = Vec::with_capacity_in(2, &alloc);
	let v2: Vec<u32, _> = Vec::with_capacity_in(5, &alloc);
	let v3: Vec<u32, _> = Vec::with_capacity_in(9, &alloc);
	assert!(alloc.is_oom());

	drop(v2);
	v1.reserve_exact(7);
	assert!(alloc.is_oom());

	drop(v3);
	v1.reserve_exact(16);
	assert!(alloc.is_oom());

	drop(v1);
	assert!(alloc.is_empty());
}

#[test]
fn test_simple_push() {
	let alloc = Stalloc::<128, 4>::new();

	let mut v: Vec<u32, _> = Vec::new_in(&alloc);
	for i in 0..128 {
		v.push(42 + i);
	}
	assert!(alloc.is_oom());
}

#[test]
fn test_boxes() {
	let alloc = Stalloc::<128, 4>::new();

	for _ in 0..128 {
		let b = Box::new_in(42, &alloc);
		mem::forget(b);
	}
	assert!(alloc.is_oom());
}

#[test]
fn self_referential() {
	let alloc = Stalloc::<256, 16>::new();

	let mut boxes = Vec::with_capacity_in(128, &alloc);
	for _ in 0..128 {
		boxes.push(Box::new_in(*b"hi there", &alloc));
	}
	assert!(alloc.is_oom());

	drop(boxes);
	assert!(alloc.is_empty());
}

#[test]
fn self_referential_growing() {
	let alloc = Stalloc::<512, 16>::new();

	let mut boxes = Vec::new_in(&alloc);
	for _ in 0..128 {
		boxes.push(Box::new_in(*b"hi there", &alloc));
	}

	drop(boxes);
	assert!(alloc.is_empty());
}

#[test]
fn grow_from_1() {
	let alloc = Stalloc::<256, 8>::new();

	let mut v = Vec::with_capacity_in(1, &alloc);
	for i in 0..256 {
		v.push(42 + i);
	}
}

#[test]
fn test_grow_and_free() {
	let alloc = Stalloc::<4, 4>::new();

	let mut v1: Vec<u32, _> = Vec::with_capacity_in(1, &alloc);
	let _v2: Vec<u32, _> = Vec::with_capacity_in(1, &alloc);
	v1.reserve_exact(2);
	let _v3: Vec<u32, _> = Vec::with_capacity_in(1, &alloc);
	assert!(alloc.is_oom());
}

#[test]
fn vec_and_growing_vec() {
	let alloc = Stalloc::<9, 4>::new();

	let mut v1: Vec<u32, _> = Vec::with_capacity_in(1, &alloc);
	v1.push(0);
	let mut v2 = Vec::with_capacity_in(4, &alloc);
	v2.push(1);
	v2.push(2);
	v2.push(3);
	v2.push(4);
	v2.push(5);

	assert!(alloc.is_oom());
}

#[test]
fn vec_and_growing_vec2() {
	let alloc = Stalloc::<14, 4>::new();

	let mut v1: Vec<u32, _> = Vec::with_capacity_in(1, &alloc);
	v1.push(0);

	let mut v2 = Vec::with_capacity_in(4, &alloc);
	v2.extend_from_slice(&[1, 2, 3, 4]);

	let mut v3: Vec<u32, _> = Vec::with_capacity_in(1, &alloc);
	v3.push(0);

	v2.extend_from_slice(&[5, 6, 7, 8]);

	let mut v4: Vec<u32, _> = Vec::with_capacity_in(4, &alloc);
	v4.extend_from_slice(&[11, 12, 13, 14]);

	assert!(alloc.is_oom());
}

#[test]
fn test_small_alloc() {
	let alloc = Stalloc::<3, 8>::new();

	let a = Box::new_in(0u8, &alloc);
	let b = Box::new_in(0u16, &alloc);
	let c = Box::new_in(0u32, &alloc);
	assert!(alloc.is_oom());

	drop(b);
	drop(a);
	drop(c);
	assert!(alloc.is_empty());
}

#[test]
fn test_large_and_small_alloc() {
	let alloc = Stalloc::<12, 4>::new();

	let a = Box::new_in(0u64, &alloc);
	let b = Box::new_in(1u128, &alloc);
	let c = Box::new_in(2u64, &alloc);

	let small1 = Box::new_in(42u8, &alloc);
	let small2 = small1.clone();
	let small3 = small1.clone();
	let small4 = small1.clone();

	assert!(alloc.is_oom());

	drop(c);
	drop(small3);
	drop(small2);
	drop(a);
	drop(small4);
	drop(small1);
	drop(b);

	assert!(alloc.is_empty());
}

#[test]
fn test_boxes_vec_grow() {
	let alloc = Stalloc::<12, 4>::new();

	let a = Box::new_in(MaybeUninit::<u32>::uninit(), &alloc);
	let b = Box::new_in(5, &alloc);
	let mut c = Vec::with_capacity_in(9, &alloc);
	drop(b);
	c.reserve_exact(10);
	c.push(1);
	drop(a);
}

#[test]
fn test_multiple_shrink() {
	let alloc = Stalloc::<24, 4>::new();

	for i in 0..24 {
		let mut v: Vec<i32, _> = Vec::with_capacity_in(24 - i, &alloc);
		v.shrink_to(1);
		mem::forget(v);
	}

	assert!(alloc.is_oom());
}

#[test]
fn test_zeroed() {
	let alloc = Stalloc::<256, 4>::new();

	let mut v: Vec<i32, _> = Vec::with_capacity_in(256, &alloc);
	v.extend_from_slice(&[0; 256]);
	assert!(v.iter().all(|&b| b == 0));

	assert!(alloc.is_oom());
}

#[test]
fn test_vec_capacity() {
	let alloc = Stalloc::<1, 1024>::new();

	let mut v: Vec<u8, _> = Vec::with_capacity_in(1, &alloc);

	for i in 0..1024u32 {
		v.push(i as u8);
	}
}

#[test]
fn test34() {
	let _a = Stalloc::<34, 4>::new();
	let _b = crate::SyncStalloc::<34, 4>::new();
}
