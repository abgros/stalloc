#[cfg(all(feature = "allocator-api", feature = "allocator-api2"))]
compile_error!("The `allocator-api` and `allocator-api2` features are mutually exclusive.");

#[cfg(not(any(feature = "allocator-api", feature = "allocator-api2")))]
/// An error type representing some kind of allocation error due to memory exhaustion.
/// This is a polyfill for `core::alloc::AllocError`, available through the nightly Allocator API.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct AllocError;

#[cfg(not(any(feature = "allocator-api", feature = "allocator-api2")))]
impl core::error::Error for AllocError {}

#[cfg(not(any(feature = "allocator-api", feature = "allocator-api2")))]
impl core::fmt::Display for AllocError {
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		f.write_str("memory allocation failed")
	}
}

#[cfg(feature = "allocator-api2")]
pub use allocator_api2::alloc::AllocError;

#[cfg(feature = "allocator-api")]
pub use core::alloc::AllocError;

#[cfg(feature = "allocator-api")]
pub use core::alloc::{Allocator, Layout};

#[cfg(feature = "allocator-api2")]
pub use allocator_api2::alloc::{Allocator, Layout};
