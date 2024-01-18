//! This module contains the `launch` function, which is the main entry point for dioxus web

pub use crate::Config;
use dioxus_core::prelude::*;
use std::any::Any;

/// Launch the web application with the given root component, context and config
///
/// For a builder API, see `LaunchBuilder` defined in the `dioxus` crate.
pub fn launch(
    root: fn() -> Element,
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any>>>,
    platform_config: Config,
) {
    wasm_bindgen_futures::spawn_local(async move {
        let mut vdom = VirtualDom::new(root);
        for context in contexts {
            vdom.insert_any_root_context(context());
        }
        crate::run(vdom, platform_config).await;
    });
}