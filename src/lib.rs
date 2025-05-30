#![deny(missing_docs)]
#![warn(missing_copy_implementations)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unused_extern_crates)]
#![warn(unused_import_braces)]
#![warn(unused_qualifications)]
#![allow(unknown_lints)]
#![allow(renamed_and_removed_lints)]
#![allow(clippy::needless_doctest_main)]
#![warn(clippy::cast_possible_truncation)]
#![warn(clippy::cast_possible_wrap)]
#![warn(clippy::cast_precision_loss)]
#![warn(clippy::cast_sign_loss)]
#![warn(clippy::enum_glob_use)]
#![warn(clippy::mut_mut)]
#![warn(clippy::mutex_integer)]
#![warn(clippy::non_ascii_literal)]
#![warn(clippy::nonminimal_bool)]
#![warn(clippy::option_unwrap_used)]
#![warn(clippy::result_unwrap_used)]
#![warn(clippy::single_match_else)]
#![warn(clippy::string_add)]
#![warn(clippy::string_add_assign)]
#![warn(clippy::unicode_not_nfc)]
#![warn(clippy::wrong_pub_self_convention)]
//#![allow(clippy::cast_possible_truncation)]
//#![allow(clippy::missing_safety_doc)]

//! Message Passing Interface bindings for Rust
//!
//! The [Message Passing Interface][MPI] (MPI) is a specification for a
//! message-passing style concurrency library. Implementations of MPI are often used to structure
//! parallel computation on High Performance Computing systems. The MPI specification describes
//! bindings for the C programming language (and through it C++) as well as for the Fortran
//! programming language. This library tries to bridge the gap into a more rustic world.
//!
//! [MPI]: http://www.mpi-forum.org
//!
//! # Usage
//!
//! Add the `mpi` crate as a dependency in your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! mpi = "0.8.0"
//! ```
//!
//! Then use it in your program like this:
//!
//! ```no_run
//! use mpi::traits::*;
//!
//! fn main() {
//!     let universe = mpi::initialize().unwrap();
//!     let world = universe.world();
//!     let size = world.size();
//!     let rank = world.rank();
//!
//!     if size != 2 {
//!         panic!("Size of MPI_COMM_WORLD must be 2, but is {}!", size);
//!     }
//!
//!     match rank {
//!         0 => {
//!             let msg = vec![4.0f64, 8.0, 15.0];
//!             world.process_at_rank(rank + 1).send(&msg[..]);
//!         }
//!         1 => {
//!             let (msg, status) = world.any_process().receive_vec::<f64>();
//!             println!(
//!                 "Process {} got message {:?}.\nStatus is: {:?}",
//!                 rank, msg, status
//!             );
//!         }
//!         _ => unreachable!(),
//!     }
//! }
//! ```
//!
//! # Features
//!
//! The bindings follow the MPI 3.1 specification.
//!
//! Currently supported:
//!
//! - **Groups, Contexts, Communicators**:
//!   - Group and (Intra-)Communicator management from section 6 is mostly complete.
//!   - no Inter-Communicators
//!   - no process topologies
//! - **Point to point communication**:
//!   - standard, buffered, synchronous and ready mode send in blocking and non-blocking variants
//!   - receive in blocking and non-blocking variants
//!   - send-receive
//!   - probe
//!   - matched probe/receive
//! - **Collective communication**:
//!   - barrier
//!   - broadcast
//!   - (all) gather
//!   - scatter
//!   - all to all
//!   - varying counts operations
//!   - reductions/scans
//!   - blocking and non-blocking variants
//! - **Datatypes**: Bridging between Rust types and MPI basic types as well as custom MPI datatypes
//! which can act as views into buffers.
//!
//! Not supported (yet):
//!
//! - One-sided communication (RMA)
//! - MPI parallel I/O
//! - A million small things
//!
//! The sub-modules contain a more detailed description of which features are and are not
//! supported.
//!
//! # Further Reading
//!
//! While every publicly defined item in this crate should have some documentation attached to it,
//! most of the descriptions are quite terse for now and to the uninitiated will only make sense in
//! combination with the [MPI specification][MPIspec].
//!
//! [MPIspec]: https://www.mpi-forum.org/docs/

use std::{mem::MaybeUninit, os::raw::c_int};

/// The raw C language MPI API
///
/// Documented in the [Message Passing Interface specification][spec]
///
/// [spec]: https://www.mpi-forum.org/docs/
#[allow(missing_docs, dead_code, non_snake_case, non_camel_case_types)]
#[macro_use]
pub mod ffi {
    pub use mpi_sys::*;
}

pub mod attribute;
pub mod collective;
pub mod datatype;
pub mod environment;
pub mod point_to_point;
pub mod raw;
pub mod request;
pub mod topology;

/// Re-exports all traits.
pub mod traits {
    // Re-export derives
    #[cfg(feature = "derive")]
    pub use mpi_derive::Equivalence;

    pub use crate::{
        attribute::traits::*, collective::traits::*, datatype::traits::*,
        point_to_point::traits::*, raw::traits::*, topology::traits::*,
    };
}

/// These crates are used by mpi-derive, and so must be public, but shouldn't be used by dependent
/// crates
#[doc(hidden)]
pub mod internal {
    #[cfg(feature = "derive")]
    pub use memoffset;
    pub use once_cell;
}

#[doc(inline)]
pub use crate::environment::{
    initialize, initialize_with_threading, time, time_resolution, Threading,
};
use crate::ffi::MPI_Aint;

/// Encodes error values returned by MPI functions.
pub type Error = c_int;
/// Encodes number of values in multi-value messages.
pub type Count = c_int;
/// Can be used to tag messages on the sender side and match on the receiver side.
pub type Tag = c_int;
/// An address in memory
pub type Address = MPI_Aint;
/// Reexport the Rank type
pub use crate::topology::Rank;

/// IntArray is used to translate Rust bool values to and from the int-bool types preferred by MPI
/// without incurring allocation in the common case.
type IntArray = smallvec::SmallVec<[c_int; 8]>;

unsafe fn with_uninitialized<F, U, R>(f: F) -> (R, U)
where
    F: FnOnce(*mut U) -> R,
{
    let mut uninitialized = MaybeUninit::uninit();
    let res = f(uninitialized.as_mut_ptr());
    (res, uninitialized.assume_init())
}

unsafe fn with_uninitialized2<F, U1, U2, R>(f: F) -> (R, U1, U2)
where
    F: FnOnce(*mut U1, *mut U2) -> R,
{
    let mut uninitialized1 = MaybeUninit::uninit();
    let mut uninitialized2 = MaybeUninit::uninit();
    let res = f(uninitialized1.as_mut_ptr(), uninitialized2.as_mut_ptr());
    (
        res,
        uninitialized1.assume_init(),
        uninitialized2.assume_init(),
    )
}

/// Errors
///
/// RSMPI is currently configured with MPI_ERRORS_ARE_FATAL, but:
///
/// 1. we intend to remove this restriction at some point
///
/// 2. we need to be able to return parse errors and it seems better to make a
/// stable error type than to propagate raw types like ``std::ffi::NulError` in
/// our public interface.
///
/// # Standard section(s)
///
/// 9.3
#[derive(thiserror::Error, Debug)]
pub enum MpiError {
    /// Failed to spawn some processes
    #[error("Failed to spawn {0} of {1} processes")]
    Spawn(Rank, Rank),
    /// CString::new fails if a Rust string contains interior 0 bytes
    #[error("An interior 0 byte was found in string")]
    StringNul(#[from] std::ffi::NulError),
}
