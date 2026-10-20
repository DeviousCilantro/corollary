//! The behaviours this module offers, in the order they are installed.

pub mod clipboard;
pub mod flow;
pub mod theme;

use crate::Plugin;

pub const ALL: &[Plugin] = &[theme::PLUGIN, clipboard::PLUGIN, flow::PLUGIN];
