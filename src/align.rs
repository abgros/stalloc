#[derive(Clone, Copy)]
#[repr(transparent)]
/// A ZST with a given alignment. `Align` and `Alignment` are used to ensure that `Block`, and hence
/// `Stalloc`, are aligned to a particular value. The definition of `Block` is:
/// ```
/// #[derive(Clone, Copy)]
/// union Block<const B: usize>
/// where
///     Align<B>: Alignment,
/// {
///     header: Header,
///     bytes: [MaybeUninit<u8>; B],
///     _align: Align<B>,
/// }
/// ```
/// This struct and trait are made public to allow you to define your own wrapper around `Stalloc`.
/// For example, `SyncStalloc` is defined as:
///
/// ```
/// pub struct SyncStalloc<const L: usize, const B: usize>
/// where
///     Align<B>: Alignment,
/// {
///     inner: Mutex<UnsafeStalloc<L, B>>,
/// }
/// ```
pub struct Align<const N: usize>(<Self as Alignment>::Inner)
where
	Self: Alignment;

/// See the documentation for `Align`.
pub trait Alignment {
	/// See the documentation for `Align`.
	type Inner: Copy;
}

macro_rules! impl_alignments {
	($($name:ident as $n:literal),*) => { $(
		#[derive(Copy, Clone)]
		#[repr(align($n))]
		/// See the documentation for `Align`.
		pub struct $name;
		impl Alignment for Align<$n> {
			type Inner = $name;
		}
	)* };
}

impl_alignments!(
	Align1 as 1, Align2 as 2, Align4 as 4, Align8 as 8, Align16 as 16, Align32 as 32,
	Align64 as 64, Align128 as 128, Align256 as 256, Align512 as 512, Align1024 as 1024,
	Align2048 as 2048, Align4096 as 4096, Align8192 as 8192, Align16384 as 16384,
	Align32768 as 32768, Align65536 as 65536, Align131072 as 131072, Align262144 as 262144,
	Align524288 as 524288, Align1048576 as 1048576, Align2097152 as 2097152,
	Align4194304 as 4194304, Align8388608 as 8388608, Align16777216 as 16777216,
	Align33554432 as 33554432, Align67108864 as 67108864, Align134217728 as 134217728,
	Align268435456 as 268435456, Align536870912 as 536870912
);
