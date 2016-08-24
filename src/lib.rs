// Copyright (c) 2015 rust-xplm developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

//! The `xplm` crate provides a convenient interface to the X-Plane plugin SDK.
//!

#![deny(missing_docs,
        missing_debug_implementations,
        trivial_casts, trivial_numeric_casts,
        unstable_features,
        unused_import_braces, unused_qualifications)]

extern crate xplm_sys;

/// Functionality for reading, writing, and creating datarefs
pub mod data;
/// Functionality for finding, creating, and executing commands
pub mod command;
/// Flight loop callbacks
pub mod flight_loop;
/// Callbacks for deferred initialization
pub mod deferred_init;
/// SDK feature querying and enabling
pub mod features;
/// Terrain probing
pub mod terrain;
/// Position types
pub mod position;

/// User interface elements
#[allow(missing_docs)]
pub mod ui;
/// Provides access to the navigation database
#[allow(missing_docs)]
pub mod nav;
/// Radio frequency representation
pub mod frequency;
/// OpenGL-related functionality
pub mod graphics;
/// Inter-plugin communication
pub mod ipc;

/// Foreign function interface utilities
mod ffi;

use std::ffi::CString;

/// Writes a message to the X-Plane log.txt file
pub fn debug(message: &str) {
    use xplm_sys::utilities::XPLMDebugString;

    match CString::new(message) {
        Ok(message_c) => unsafe { XPLMDebugString(message_c.as_ptr()) },
        Err(_) => unsafe {
            XPLMDebugString(b"xplm::debug: Provided string not valid"
                                .as_ptr() as *const ffi::types::c_char)
        },
    }
}

/// Enables the logging of plugin-related errors to the log.txt file
pub fn enable_debug_logging() {
    unsafe { xplm_sys::utilities::XPLMSetErrorCallback(Some(log_callback)) };
}

/// The error callback provided to X-Plane that receives error messages
unsafe extern "C" fn log_callback(message: *const ffi::types::c_char) {
    use std::ffi::CStr;
    debug(&format!("XPLM error: {}\n",
                   CStr::from_ptr(message).to_string_lossy().into_owned()));
}

/// Finds a symbol in the set of currently loaded libraries
pub fn find_symbol(name: &str) -> *mut ffi::types::c_void {
    use std::ptr;
    use xplm_sys::utilities::XPLMFindSymbol;

    match CString::new(name) {
        Ok(name_c) => unsafe { XPLMFindSymbol(name_c.as_ptr()) },
        Err(_) => ptr::null_mut(),
    }
}

/// Stores information about a plugin that is provided to X-Plane
#[derive(Debug, Clone)]
pub struct PluginInfo {
    /// The plugin name
    pub name: &'static str,
    /// The plugin's signature, in reverse DNS format
    pub signature: &'static str,
    /// A description of the plugin
    pub description: &'static str,
}


/// The trait that all plugins should implement
pub trait Plugin : Sized {
    /// Called when X-Plane loads this plugin
    ///
    /// On success, returns a plugin object.
    ///
    /// On failure, returns an error.
    fn start() -> Result<Self, Box<std::error::Error>>;
    /// Called when the plugin is enabled
    fn enable(&mut self);
    /// Called when the plugin is disabled
    fn disable(&mut self);

    /// Returns information on this plugin
    fn info() -> PluginInfo;

    /// Called when a plugin sends a message to this plugin
    ///
    /// The message number and the plugin that sent the message are provided. This method is not
    /// called when X-Plane sends a message.
    fn message_from_plugin(&mut self, message: i32, from: ipc::Plugin);

    /// Called when X-Plane sends a message to this plugin
    ///
    /// The message is provided.
    fn message_from_xplane(&mut self, message: ipc::XPlaneMessage);

    // Called when the plugin is stopped
    ///
    /// The plugin will be droped after this function is called.
    fn stop(&mut self);
}

/// Creates an X-Plane plugin
///
/// Provide the name of your plugin struct. The callbacks that X-Plane uses will be created.
///
#[macro_export]
macro_rules! xplane_plugin {
    ($plugin_type: ty) => (
        use ::std::ptr;
        use ::std::ffi;
        type PluginType = $plugin_type;
        type PluginPtr = *mut PluginType;
        // The plugin
        static mut PLUGIN: PluginPtr = 0 as PluginPtr;

        #[allow(non_snake_case)]
        #[no_mangle]
        pub extern "C" fn XPluginStart(outName: *mut ::xplm::ffi::types::c_char, outSig: *mut ::xplm::ffi::types::c_char,
            outDescription: *mut ::xplm::ffi::types::c_char) -> ::xplm::ffi::types::c_int
        {
            // Create the plugin, temporarily, on the stack
            let plugin_option = PluginType::start();
            match plugin_option {
                Some(plugin) => {
                    // Allocate storage
                    PLUGIN = unsafe { Box::into_raw(Box::new(plugin)); }

                    let info = (*PLUGIN).info();

                    match ffi::CString::new(info.name).ok() {
                        Some(name) => ::xplm::ffi::types::strcpy(outName, name.as_ptr()),
                        None => ::xplm::ffi::types::strcpy(outName, b"<invalid>".as_ptr() as *const ::xplm::ffi::types::c_char),
                    };
                    match ffi::CString::new(info.signature).ok() {
                        Some(signature) => ::xplm::ffi::types::strcpy(outSig, signature.as_ptr()),
                        None => ::xplm::ffi::types::strcpy(outSig, b"<invalid>".as_ptr() as *const ::xplm::ffi::types::c_char),
                    };
                    match ffi::CString::new(info.description).ok() {
                        Some(description) => ::xplm::ffi::types::strcpy(outDescription, description.as_ptr()),
                        None => ::xplm::ffi::types::strcpy(outDescription, b"<invalid>".as_ptr() as *const ::xplm::ffi::types::c_char),
                    };

                    // Success
                    1
                },
                None => {
                    // Return failure
                    0
                },
            }
        }

        #[allow(non_snake_case)]
        #[no_mangle]
        pub unsafe extern "C" fn XPluginStop() {
            (*PLUGIN).stop();
            // Free plugin
            let plugin_box = Box::from_raw(PLUGIN);
            drop(plugin_box);
            PLUGIN = ptr::null_mut();
        }

        #[allow(non_snake_case)]
        #[no_mangle]
        pub unsafe extern "C" fn XPluginEnable() {
            (*PLUGIN).enable();
        }

        #[allow(non_snake_case)]
        #[no_mangle]
        pub unsafe extern "C" fn XPluginDisable() {
            (*PLUGIN).disable();
        }

        #[allow(non_snake_case)]
        #[no_mangle]
        pub unsafe extern "C" fn XPluginReceiveMessage(inFrom: ::xplm::ffi::types::c_int, inMessage: ::xplm::ffi::types::c_int,
            _: *mut ::xplm::ffi::types::c_void)
        {
            if inFrom == ipc::XPLANE_ID {
                if let Some(message) = ::xplm::ipc::XPlaneMessage::from_i32(inMessage) {
                    (*PLUGIN).message_from_xplane(message);
                }
            } else {
                let sender = ipc::Plugin::with_id(inFrom);
                (*PLUGIN).message_from_plugin(inMessage, sender);
            }
        }
    )
}
