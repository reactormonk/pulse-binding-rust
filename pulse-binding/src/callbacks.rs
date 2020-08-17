// Copyright 2018 Lyndon Brown
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

//! Callback handling.

use std::os::raw::c_void;
use std::ptr::null_mut;
use std::marker::PhantomData;

/// List result instance.
///
/// Fetching a list can result in a callback being fired for each list item, and then once to signal
/// that the end of the list having been reached. This is used to distinguish such state to a
/// closure callback.
#[derive(Debug, Clone)]
pub enum ListResult<T> {
    /// List item
    Item(T),
    /// End of list reached
    End,
    /// Failure, an error occurred
    Error,
}

/// A saved multi-use callback.
///
/// Closures of multi-use callbacks (those that may be called multiple times) need saving, and
/// releasing later at an appropriate time (on change of registered callback, or on destruction of
/// associated object). This is used for saving the pointer to it for such deferred destruction.
pub(crate) struct MultiUseCallback<ClosureProto: ?Sized, ProxyProto> {
    saved: Option<*mut Box<ClosureProto>>,
    proxy: PhantomData<*const ProxyProto>,
}

impl<ClosureProto: ?Sized, ProxyProto> Default for MultiUseCallback<ClosureProto, ProxyProto> {
    fn default() -> Self {
        MultiUseCallback::<ClosureProto, ProxyProto> { saved: None, proxy: PhantomData }
    }
}

impl<ClosureProto: ?Sized, ProxyProto> MultiUseCallback<ClosureProto, ProxyProto> {
    /// Creates a new instance.
    ///
    /// **Note**, an existing instance should always be overwritten with a new one, to ensure the
    /// old one is correctly freed.
    pub fn new(cb: Option<Box<ClosureProto>>) -> Self {
        match cb {
            Some(f) => MultiUseCallback::<ClosureProto, ProxyProto> {
                saved: Some(Box::into_raw(Box::new(f))),
                proxy: PhantomData,
            },
            None => Default::default(),
        }
    }

    /// Returns callback params to give to the C API (a tuple of function pointer and data pointer).
    pub fn get_capi_params(&self, proxy: ProxyProto) -> (Option<ProxyProto>, *mut c_void) {
        match self.saved {
            Some(ref f) => (Some(proxy), *f as *mut c_void),
            None => (None, null_mut::<c_void>()),
        }
    }

    /// Converts void closure pointer back to real type.
    ///
    /// For use in callback proxies.
    ///
    /// Only a reference is returned, in order to deliberately avoid reclaiming ownership and thus
    /// triggering of destruction.
    ///
    /// Panics if `ptr` is null.
    pub fn get_callback<'a>(ptr: *mut c_void) -> &'a mut Box<ClosureProto> {
        assert!(!ptr.is_null());
        // Note, does NOT destroy closure callback after use - only handles pointer
        unsafe { &mut *(ptr as *mut Box<ClosureProto>) }
    }
}

impl<ClosureProto: ?Sized, ProxyProto> Drop for MultiUseCallback<ClosureProto, ProxyProto> {
    fn drop(&mut self) {
        if self.saved.is_some() {
            let _to_drop = unsafe { Box::from_raw(self.saved.unwrap()) };
        }
    }
}

/// Converts single-use-callback closure to pointer for C API.
///
/// It can be restored in an `extern "C"` callback proxy with `get_su_callback`.
///
/// Note: The closure itself needs to exist on the heap, and here we take it as such (wrapped in a
/// `Box`); from this you may assume that a pointer is already available. However, this is a pointer
/// to a trait object, and these are special in Rust, they are twice the size of a normal pointer
/// because in actual fact it is implemented as a pair of pointers (one to a type instance and one
/// to a ‘vtable’). We can only pass a normal sized pointer through the C API, so we must further
/// box it, producing `Box<Box<Closure>>` which we convert to `*mut Box<Closure>` and then further
/// to simply `*mut c_void`.
pub(crate) fn box_closure_get_capi_ptr<ClosureProto: ?Sized>(callback: Box<ClosureProto>)
    -> *mut c_void
{
    Box::into_raw(Box::new(callback)) as *mut c_void
}

/// Gets the C API callback params (function pointer and data pointer pair), for an optional
/// single-use callback closure.
///
/// The proxy function must be specified. If `callback` is `None` then a pair of null pointers will
/// be returned. Otherwise, a pair consisting of the given proxy and a pointer for the given closure
/// will be returned. The data pointer can be restored to the actual (boxed) closure in the
/// `extern "C"` callback proxy with `get_su_callback`.
pub(crate) fn get_su_capi_params<ClosureProto: ?Sized, ProxyProto>(
    callback: Option<Box<ClosureProto>>, proxy: ProxyProto) -> (Option<ProxyProto>, *mut c_void)
{
    match callback {
        Some(f) => (Some(proxy), box_closure_get_capi_ptr::<ClosureProto>(f)),
        None => (None, null_mut::<c_void>()),
    }
}

/// Converts void single-use-callback closure pointer back to real type.
///
/// For use in callback proxies.
///
/// Returns ownership of the closure, thus it can be destroyed after use.
///
/// Panics if `ptr` is null.
pub(crate) fn get_su_callback<ClosureProto: ?Sized>(ptr: *mut c_void) -> Box<Box<ClosureProto>> {
    assert!(!ptr.is_null());
    unsafe { Box::from_raw(ptr as *mut Box<ClosureProto>) }
}

pub(crate) enum ListInstanceCallback<'a, ClosureProto: 'a + ?Sized> {
    /// An entry instance. Contains reference to closure callback.
    Entry(&'a mut Box<ClosureProto>),
    /// End-of-list instance. Contains owned closure callback, for destruction after use.
    End(Box<Box<ClosureProto>>),
    /// Error instance. Contains owned closure callback, for destruction after use.
    Error(Box<Box<ClosureProto>>),
}

/// Used by multi-use-list style callback proxies.
///
/// Provide this with the `eol` parameter, and the userdata (closure) pointer parameter, and it will
/// return either a reference to the closure or the owned closure, depending upon whether or not
/// `eol` signals end-of-list/error.
pub(crate) fn callback_for_list_instance<'a, ClosureProto: ?Sized>(eol: i32, ptr: *mut c_void)
    -> ListInstanceCallback<'a, ClosureProto>
{
    assert!(!ptr.is_null());
    match eol {
        0 => { // NOT end-of-list or error. Return reference to avoid destruction.
            let callback = unsafe { &mut *(ptr as *mut Box<ClosureProto>) };
            ListInstanceCallback::Entry(callback)
        },
        i => { // End-of-list or error. Return owned, so it can be destroyed after use.
            let callback = unsafe { Box::from_raw(ptr as *mut Box<ClosureProto>) };
            match i > 0 {
                true => ListInstanceCallback::End(callback),
                false => ListInstanceCallback::Error(callback),
            }
        },
    }
}
