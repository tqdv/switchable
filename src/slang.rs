// This where the magic happens… or something
//
// # Usage
// ```rust
// #[macro_use] mod util; // In your entrypoint file
// prelude!(); // Import symbols
//
// terror! { f() => |e| handle_f(e) }
// ```
//
// # Description
// - Metadata tuple of (Meta, Data)
// - ValRet enum of Val | Ret. Used with ret! to emit a "return control code"
//
// # Symbols
// - Metadata
// - ValRet
// - tear!, rip!, fear!, terror!
// - tear_if!, match_ok!, match_some!
//
// # Ressources
// - https://medium.com/@phoomparin/a-beginners-guide-to-rust-macros-5c75594498f1
// - Other related words: crease, cut, fold, split, strip
//   See https://words.bighugelabs.com/tear
#![allow(dead_code)] // FIXME, TEMP
#![allow(unused_macros)] // FIXME, TEMP

pub struct Metadata<Meta, Data>
(
	pub Meta,
	pub Data
);

pub enum ValRet<V, R> {
	Val(V),
	Ret(R),
}

impl<V, R> ValRet<V, R> {
	fn valret (self) -> Self {
		self
	}
	
	fn map_ret<R1> (self, f :impl FnOnce(R) -> R1) -> ValRet<V, R1> {
		match self {
			Val(v) => Val(v),
			Ret(r) => Ret(f(r)),
		}
	}
	
	fn or_else<R1> (self, f :impl FnOnce(R) -> ValRet<V, R1>) -> ValRet<V, R1> {
		match self {
			Val(v) => Val(v),
			Ret(r) => f(r),
		}
	}
}

macro_rules! tear {
	($f:expr) => {
		match $f.valret() {
			ValRet::Val(v) => v,
			ValRet::Ret(r) => return r,
		}
	}
}

macro_rules! tear_if {
	( $c:expr, $($b:expr);* ) => {
		tear! {
			if $c {
				ValRet::Ret({ $($b);* })
			} else {
				ValRet::Val(())
			}
		}
	}
}

pub type Ret<R> = ValRet<(), R>;

pub trait Return<T, R> where Self :Sized {		
	// Moves the good value into Val and the bad value into Ret
	fn valret(self) -> ValRet<T, R>;
	
	// Or else create a new ValRet from Ret
	fn val_or_else<R1> (self, f :impl FnOnce(R) -> ValRet<T, R1>) -> ValRet<T, R1> {
		self.valret().or_else(f)
	}
	
	// Or map Ret to a new Ret
	fn val_or_map<R1> (self, f :impl FnOnce(R) -> R1) -> ValRet<T, R1> {
		self.valret().map_ret(f)
	}
	
	// Or Ret produced by function
	fn val_or_ret<R1> (self, f :impl FnOnce() -> R1) -> ValRet<T, R1> {
		self.valret().map_ret(|_| f())
	}
}

impl<T, E, Me> Return<T, E> for Me where Me :Judge<Positive=T, Negative=E> + Sized {
	fn valret (self) -> ValRet<T, E> {
		self.into_moral().into_valret()
	}
}

macro_rules! match_ok {
	[ $m:expr ; $( $p:pat => $e:expr ),* $(,)? ] => {
		match $m {
			Ok(v) => v,
			$( $p => $e, )*
		}
	}
}

macro_rules! match_some {
	[ $m:expr ; $( $p:pat => $e:expr ),* $(,)? ] => {
		match $m {
			Some(v) => v,
			$( $p => $e, )*
		}
	}
}

// Extensions for normal types

pub trait IterExt<T> {
	// T: iterator value
	fn reduce (self, f :impl FnMut(T, T) -> T) -> Option<T>;
}

impl<T, Me> IterExt<T> for Me where Me :Iterator<Item=T> {
	fn reduce (mut self, f :impl FnMut(T, T) -> T) -> Option<T> {
		self.next().map(|first| self.fold(first, f))
	}
}

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

// [[ Let's reinvent Result/Option and Try ]]
use Moral::*;

pub enum Moral<Y, N> {
	Good(Y),
	Bad(N)
}

// Mirrors Try trait
pub trait Judge {
	type Positive;
	type Negative;
	
	fn into_moral(self) -> Moral<Self::Positive, Self::Negative>;
	fn from_good (v :Self::Positive) -> Self;
	fn from_bad (v :Self::Negative) -> Self;
}

// # The useful macros

// rip is like tear, but stronger and more righteous
// It automatically coerces Ret to the return type bad value
// That conversion only exists when the return type has a notion of good and bad (Try trait)
// cf. https://blog.burntsushi.net/rust-error-handling/#the-real-try-macro-operator
macro_rules! rip {
	($f:expr) => {
		tear! { $f.into_moral().ret_error() }
	}
}

// When you need to scream an error from the inside
// Used when error handling with a closure would move variables
// too early for the borrow checker. Or when you want to be explicit with errors
macro_rules! terror {
	($e:expr => $f:expr) => {
		{
			#[allow(clippy::redundant_closure_call)]
			match $e.into_moral() {
				Moral::Good(v) => v,
				Moral::Bad(v) => return Judge::from_bad(($f)(v)),
			}
		}
	}
}

// When you need to shoot it through a map cannon
// Used when you want to map the bad value to the return type, but don't want to use
// `tear! { $.val_or_map(f) }` to be more explicit
macro_rules! fear {
	($e:expr => $f:expr) => {
		{
			#[allow(clippy::redundant_closure_call)]
			match $e.into_moral() {
				Moral::Good(v) => v,
				Moral::Bad(v) => return (($f)(v)),
			}
		}
	}
}

// Methods on Moral
impl<Y, N> Moral<Y, N> {
	pub fn into_valret (self) -> ValRet<Y, N> {
		match self {
			Good(v) => Val(v),
			Bad(v) => Ret(v),
		}
	}
	
	pub fn into_result (self) -> Result<Y, N> {
		match self {
			Good(v) => Ok(v),
			Bad(v) => Err(v),
		}
	}
	
	pub fn good (self) -> Option<Y> {
		match self {
			Good(v) => Some(v),
			Bad(_) => None,
		}
	}
	
	// Used when the good value isn't the same as the function return good value
	// `tear! { $.val_adapt() }` mimics $?
	// It converts from Try<Ok=T, Error=E> (statement value)
	// to Try<Ok=O, Error=E> (function value)
	pub fn ret_error<O, R :Judge<Positive=O, Negative=N>> (self) -> ValRet<Y, R> {
		match self {
			Good(v) => Val(v),
			Bad(v) => Ret(Judge::from_bad(v)),
		}
	}
}

// Implementations for Option and Result, more or less copy-paste
impl<T> Judge for Option<T> {
	type Positive = T;
	type Negative = ();

	// #[inline]
	fn into_moral (self) -> Moral<T, ()> {
		match self {
			Some(v) => Good(v),
			None => Bad(()),
		}
	}

	#[inline]
	fn from_good(v :T) -> Self {
		Some(v)
	}

	#[inline]
	fn from_bad(_ :()) -> Self {
		None
	}
}

impl<T, E> Judge for Result<T, E> {
	type Positive = T;
	type Negative = E;

	// #[inline]
	fn into_moral (self) -> Moral<T, E> {
		match self {
			Ok(v) => Good(v),
			Err(e) => Bad(e),
		}
	}

	#[inline]
	fn from_good (v :T) -> Self {
		Ok(v)
	}

	#[inline]
	fn from_bad (v :E) -> Self {
		Err(v)
	}
}

// Implementation for ValRet and Moral
impl<T, R> Judge for ValRet<T, R> {
	type Positive = T;
	type Negative = R;

	fn into_moral (self) -> Moral<T, R> {
		match self {
			Val(v) => Good(v),
			Ret(r) => Bad(r),
		}
	}

	#[inline]
	fn from_good (v :T) -> Self {
		Val(v)
	}

	#[inline]
	fn from_bad (r :R) -> Self {
		Ret(r)
	}
}

impl<Y, N> Judge for Moral<Y, N> {
	type Positive = Y;
	type Negative = N;

	#[inline]
	fn into_moral (self) -> Moral<Y, N> {
		self
	}

	#[inline]
	fn from_good (v :Y) -> Self {
		Good(v)
	}

	#[inline]
	fn from_bad (v :N) -> Self {
		Bad(v)
	}
}
// End Judge implementation

//[[ End reinvention ]]

pub use ValRet::*;

macro_rules! prelude {
	() => {
		use crate::slang::*;
		#[allow(unused_imports)]
		use crate::exitcode::{self, ExitCode};
	}
}