// Evil trait magic to define a ZST with a specified alignment, e.g. Align<32>

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Align<const N: usize>(<Self as Alignment>::Inner)
where
	Self: Alignment;

pub trait Alignment {
	type Inner: Copy;
}

macro_rules! impl_alignments {
	($($name:ident as $n:literal),*) => { $(
		#[derive(Copy, Clone)]
		#[repr(align($n))]
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
