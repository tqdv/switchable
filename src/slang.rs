//! Syntax and type extensions

/// Represent data, and its metadata
pub struct Metadata<Meta, Data>
(
	pub Meta,
	pub Data
);

// Extensions for normal types

/// Add the reduce method to iterators
pub trait IterExt<T> {
	// T: iterator value
	fn reduce (self, f :impl FnMut(T, T) -> T) -> Option<T>;
}

impl<T, Me> IterExt<T> for Me where Me :Iterator<Item=T> {
	fn reduce (mut self, f :impl FnMut(T, T) -> T) -> Option<T> {
		self.next().map(|first| self.fold(first, f))
	}
}

/// Split an `Option<(_, _)>` into two
pub trait Split2<T1, T2> {
	fn split2 (self) -> (Option<T1>, Option<T2>);
}

impl<T1, T2> Split2<T1, T2> for Option<(T1, T2)> {
	fn split2 (self) -> (Option<T1>, Option<T2>) {
		match self {
			Some((a, b)) => (Some(a), Some(b)),
			None => (None, None),
		}
	}
}

/// Prelude for the crate: syntax extensions
macro_rules! prelude {
	() => {
		#[allow(unused_imports)] use crate::slang::*;
		#[allow(unused_imports)] use tear::prelude::*;
		#[allow(unused_imports)] use thiserror::Error as ThisError;
	}
}
