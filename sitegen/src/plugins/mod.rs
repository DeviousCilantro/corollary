//! Plugins: self-contained features that can be switched on or off in
//! `[plugins]` of content/site.toml.
//!
//! There are two kinds, and one table switches both:
//!
//! - **Build plugins** run here, in the generator. Each implements
//!   [`BuildPlugin`] and may rewrite the files copied from `static/` (before
//!   their URLs are fingerprinted) or decorate each publication before it is
//!   rendered.
//! - **Runtime plugins** run in the reader's browser, in the WebAssembly module
//!   (`wasm/src/plugins/`). The generator only names the enabled ones in the
//!   page, as `<html data-plugins="…">`, and the module installs those.
//!
//! To add a build plugin: write a module here with a type implementing
//! [`BuildPlugin`], give it a switch in [`crate::config::Plugins`], and push it
//! in [`Plugins::new`].

mod bibtex_highlight;
mod minify_css;

use crate::config::{self, Features};
use crate::publications::PublicationView;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;


/// A feature that runs at build time. Every hook has a default that does
/// nothing, so a plugin implements only what it needs.
pub trait BuildPlugin {
    /// Rewrites a file copied from `static/`, given its path relative to the
    /// output directory. `None` leaves the file as it is.
    fn asset(&self, _path: &Path, _contents: &[u8]) -> Option<Vec<u8>> {
        None
    }

    /// Decorates one publication or manuscript before it is rendered.
    fn publication(&self, _item: &mut PublicationView) {}
}

/// The plugins this build runs, and the names of those the page runs.
pub struct Plugins {
    build: Vec<Box<dyn BuildPlugin>>,
    runtime: Vec<&'static str>,
}

impl Plugins {
    pub fn new(switches: &config::Plugins, features: &Features) -> Self {
        let mut build: Vec<Box<dyn BuildPlugin>> = Vec::new();
        if switches.minify_css {
            build.push(Box::new(minify_css::MinifyCss));
        }
        if switches.bibtex_highlight {
            build.push(Box::new(bibtex_highlight::BibtexHighlight));
        }

        let mut runtime = Vec::new();
        if features.theme_toggle {
            runtime.push("theme");
        }
        runtime.push("clipboard");
        if switches.heading_flow {
            runtime.push("flow");
        }
        Self { build, runtime }
    }

    /// Runs every build plugin over the files already copied into `public`.
    pub fn process_assets(&self, public: &Path) -> Result<()> {
        if self.build.is_empty() {
            return Ok(());
        }
        for entry in WalkDir::new(public) {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }
            let rel = entry.path().strip_prefix(public)?;
            let original =
                fs::read(entry.path()).with_context(|| format!("reading {}", rel.display()))?;
            let mut contents = original.clone();
            for plugin in &self.build {
                if let Some(changed) = plugin.asset(rel, &contents) {
                    contents = changed;
                }
            }
            if contents != original {
                fs::write(entry.path(), contents)
                    .with_context(|| format!("writing {}", rel.display()))?;
            }
        }
        Ok(())
    }

    pub fn process_publications(&self, items: &mut [PublicationView]) {
        for item in items {
            for plugin in &self.build {
                plugin.publication(item);
            }
        }
    }

    /// The runtime plugins to install, as the page's `data-plugins` attribute.
    pub fn runtime_list(&self) -> String {
        self.runtime.join(" ")
    }
}
