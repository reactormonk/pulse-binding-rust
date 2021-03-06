// Copyright 2017 Lyndon Brown
//
// This file is part of the PulseAudio Rust language binding.
//
// Licensed under the MIT license or the Apache license (version 2.0), at your option. You may not
// copy, modify, or distribute this file except in compliance with said license. You can find copies
// of these licenses either in the LICENSE-MIT and LICENSE-APACHE files, or alternatively at
// <http://opensource.org/licenses/MIT> and <http://www.apache.org/licenses/LICENSE-2.0>
// respectively.
//
// Portions of documentation are copied from the LGPL 2.1+ licensed PulseAudio C headers on a
// fair-use basis, as discussed in the overall project readme (available in the git repository).

//! Time handling functionality.

mod microseconds;
mod monotonic;
mod timeval;
mod unix;

use std::time::Duration;

pub use self::microseconds::*;
pub use self::monotonic::*;
pub use self::timeval::*;
pub use self::unix::*;

// (Copied constants from rust’s std/time/duration.rs)
pub const NANOS_PER_SEC: u32 = 1_000_000_000;
pub const NANOS_PER_MILLI: u32 = 1_000_000;
pub const NANOS_PER_MICRO: u32 = 1_000;
pub const MICROS_PER_SEC: u64 = 1_000_000;
pub const MICROS_PER_MILLI: u64 = 1_000;
pub const MILLIS_PER_SEC: u64 = 1_000;

/// Invalid time. Microseconds value representing ‘invalid’.
pub const USEC_INVALID: MicroSeconds = MicroSeconds(capi::PA_USEC_INVALID);

/// Largest valid time value in microseconds (largest integer value is reserved for representing
/// ‘invalid’).
pub const USEC_MAX: MicroSeconds = MicroSeconds(capi::PA_USEC_MAX);

impl From<Timeval> for MicroSeconds {
    #[inline]
    fn from(t: Timeval) -> Self {
        MicroSeconds(unsafe { capi::pa_timeval_load(&t.0) })
    }
}
impl From<MicroSeconds> for Timeval {
    #[inline]
    fn from(t: MicroSeconds) -> Self {
        let secs = t.0 / MICROS_PER_SEC;
        let usecs = t.0 % MICROS_PER_SEC;
        Timeval::new(secs as self::timeval::TvSecs, usecs as self::timeval::TvUsecs)
    }
}

impl From<Duration> for MicroSeconds {
    #[inline]
    fn from(t: Duration) -> Self {
        MicroSeconds((t.as_secs() * MILLIS_PER_SEC) + t.subsec_millis() as u64)
    }
}
impl From<MicroSeconds> for Duration {
    #[inline]
    fn from(t: MicroSeconds) -> Self {
        Duration::from_millis(t.0)
    }
}

impl From<Duration> for Timeval {
    #[inline]
    fn from(t: Duration) -> Self {
        Timeval::new(t.as_secs() as self::timeval::TvSecs, t.subsec_millis() as self::timeval::TvUsecs)
    }
}
impl From<Timeval> for Duration {
    #[inline]
    fn from(t: Timeval) -> Self {
        Duration::from_millis((MicroSeconds::from(t)).0)
    }
}
