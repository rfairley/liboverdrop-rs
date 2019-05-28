use crate::config::fragments;
use failure::{Fallible, ResultExt};
use log::{debug, trace};
use serde::Serialize;

/// Runtime configuration holding environmental inputs.
#[derive(Debug, Serialize)]
pub(crate) struct ConfigInput {
    pub(crate) cincinnati: CincinnatiInput,
    pub(crate) updates: UpdateInput,
    pub(crate) identity: IdentityInput,
}

impl ConfigInput {
    /// Read config fragments and merge them into a single config.
    pub(crate) fn read_configs(dirs: &[&str], app_name: &str) -> Fallible<Self> {
        use std::io::Read;

        let mut fragments = Vec::new();
        for prefix in dirs {
            let dir = format!("{}/{}/config.d", prefix, app_name);
            debug!("scanning configuration directory '{}'", dir);

            let wildcard = format!("{}/*.toml", dir);
            let toml_files = glob::glob(&wildcard)?;
            for fpath in toml_files.filter_map(Result::ok) {
                trace!("reading config fragment '{}'", fpath.display());

                let fp = std::fs::File::open(&fpath)
                    .context(format!("failed to open file '{}'", fpath.display()))?;
                let mut bufrd = std::io::BufReader::new(fp);
                let mut content = vec![];
                bufrd
                    .read_to_end(&mut content)
                    .context(format!("failed to read content of '{}'", fpath.display()))?;
                let frag: fragments::ConfigFragment =
                    toml::from_slice(&content).context("failed to parse TOML")?;

                fragments.push(frag);
            }
        }

        let cfg = Self::merge_fragments(fragments);
        Ok(cfg)
    }
}

use super::errors;
use std::{collections, fs, path};

pub struct OverdropConf {
    dirs: Vec<path::PathBuf>,
}

impl OverdropConf {
    pub fn new(basedirs: &[path::PathBuf], reldir: &str, version: Option<u32>) -> Self {
        let mut dirs = Vec::with_capacity(basedirs.len());
        let ver = version.unwrap_or(0);
        for bdir in basedirs {
            let mut dpath = path::PathBuf::from(bdir);
            dpath.push(reldir.clone());
            dirs.push(dpath);
            if ver > 0 {
                dirs.push(format!("v{}", ver).to_owned().into());
            }
        }
        Self { dirs }
    }

    pub fn scan_unique_files(
        &self,
    ) -> errors::Result<collections::BTreeMap<String, path::PathBuf>> {
        let mut files_map = collections::BTreeMap::new();
        for dir in &self.dirs {
            let dir_iter = match fs::read_dir(dir) {
                Ok(iter) => iter,
                _ => continue,
            };
            for dir_entry in dir_iter {
                let entry = match dir_entry {
                    Ok(f) => f,
                    _ => continue,
                };
                let fpath = entry.path();
                let fname = entry.file_name().into_string().unwrap();

                // Ignore dotfiles.
                if fname.starts_with('.') {
                    continue;
                };
                // Ignore non-TOML.
                if !fname.ends_with(".toml") {
                    continue;
                };

                // Check filetype, ignore non-file.
                let meta = match entry.metadata() {
                    Ok(m) => m,
                    _ => continue,
                };
                if !meta.file_type().is_file() {
                    if let Ok(target) = fs::read_link(&fpath) {
                        // A devnull symlink is a special case to ignore previous file-names.
                        if target == path::PathBuf::from("/dev/null") {
                            trace!("Nulled config file '{}'", fpath.display());
                            files_map.remove(&fname);
                        }
                    }
                    continue;
                }

                // TODO(lucab): return something smarter than a PathBuf.
                trace!("Found config file '{}' at '{}'", fname, fpath.display());
                files_map.insert(fname, fpath);
            }
        }
        Ok(files_map)
    }
}