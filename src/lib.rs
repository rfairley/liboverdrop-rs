#[macro_use]
extern crate error_chain;

mod errors {
    error_chain!{}
}

use log::trace;
use std::{collections, fs, path};

pub struct OverdropConf {
    dirs: Vec<path::PathBuf>,
}

impl OverdropConf {
    pub fn new(
        base_dirs: &Vec<String>,
        config_path: &str,
    ) -> Self {
        let mut dirs = Vec::with_capacity(base_dirs.len());
        for bdir in base_dirs {
            let mut dpath = path::PathBuf::from(bdir);
            dpath.push(config_path.clone());
            dirs.push(dpath);
        }
        Self { dirs }
    }

    pub fn new_full(
        root_dir: &str,
        base_dirs: &Vec<String>,
        config_path: &str,
        version: Option<String>,
    ) -> Self {
        let mut dirs = Vec::with_capacity(base_dirs.len());
        let version = version.unwrap_or("".to_string());
        for bdir in base_dirs {
            let mut dpath = path::PathBuf::from(root_dir);
            dpath.push(bdir);
            dpath.push(format!("{}{}", config_path, version));
            dirs.push(dpath);
        }
        Self { dirs }
    }

    // TODO: add options to exclude/include prefix/file suffix or glob (like in https://github.com/overdrop/overdrop-sebool/blob/master/src/od_cfg.rs#L42)

    pub fn scan_unique_files(
        &self,
    ) -> errors::Result<collections::BTreeMap<String, path::PathBuf>> {
        let mut files_map = collections::BTreeMap::new();
        for dir in &self.dirs {
            trace!("Scanning directory '{}'", dir.to_str().unwrap());

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

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_tree_snippet_match(
        fragments: &collections::BTreeMap<String, path::PathBuf>,
        prefix: &str,
        filename: &str,
        relpath: &str,
    ) -> () {
        assert_eq!(
            fragments
                .get(&String::from(filename))
                .unwrap()
                .strip_prefix(prefix)
                .unwrap(),
            &path::PathBuf::from(relpath)
        );
    }

    #[test]
    fn basic_override() {
        let treedir = "tests/fixtures/tree-basic";
        let dirs = vec![
            format!("{}/{}", treedir, "usr/lib"),
            format!("{}/{}", treedir, "run"),
            format!("{}/{}", treedir, "etc"),
        ];

        let od_cfg = OverdropConf::new(&dirs, "liboverdrop-rs/config.d-v0.1.0");
        let fragments = od_cfg.scan_unique_files().unwrap();

        assert_tree_snippet_match(&fragments, treedir, "01-config-a.toml", "etc/liboverdrop-rs/config.d-v0.1.0/01-config-a.toml");
        assert_tree_snippet_match(&fragments, treedir, "02-config-b.toml", "run/liboverdrop-rs/config.d-v0.1.0/02-config-b.toml");
        assert_tree_snippet_match(&fragments, treedir, "03-config-c.toml", "etc/liboverdrop-rs/config.d-v0.1.0/03-config-c.toml");
        assert_tree_snippet_match(&fragments, treedir, "04-config-d.toml", "usr/lib/liboverdrop-rs/config.d-v0.1.0/04-config-d.toml");
        assert_tree_snippet_match(&fragments, treedir, "05-config-e.toml", "etc/liboverdrop-rs/config.d-v0.1.0/05-config-e.toml");
        assert_tree_snippet_match(&fragments, treedir, "06-config-f.toml", "run/liboverdrop-rs/config.d-v0.1.0/06-config-f.toml");
        assert_tree_snippet_match(&fragments, treedir, "07-config-g.toml", "etc/liboverdrop-rs/config.d-v0.1.0/07-config-g.toml");
    }

    #[test]
    fn basic_override_new_full() {
        let treedir = "tests/fixtures/tree-basic";
        let dirs = vec![
            "usr/lib".to_string(),
            "run".to_string(),
            "etc".to_string(),
        ];
        let config_path = "liboverdrop-rs/config.d";
        let version = Some("-v0.1.0".to_string());

        let od_cfg = OverdropConf::new_full(treedir, &dirs, config_path, version);
        let fragments = od_cfg.scan_unique_files().unwrap();

        assert_tree_snippet_match(&fragments, treedir, "01-config-a.toml", "etc/liboverdrop-rs/config.d-v0.1.0/01-config-a.toml");
        assert_tree_snippet_match(&fragments, treedir, "02-config-b.toml", "run/liboverdrop-rs/config.d-v0.1.0/02-config-b.toml");
        assert_tree_snippet_match(&fragments, treedir, "03-config-c.toml", "etc/liboverdrop-rs/config.d-v0.1.0/03-config-c.toml");
        assert_tree_snippet_match(&fragments, treedir, "04-config-d.toml", "usr/lib/liboverdrop-rs/config.d-v0.1.0/04-config-d.toml");
        assert_tree_snippet_match(&fragments, treedir, "05-config-e.toml", "etc/liboverdrop-rs/config.d-v0.1.0/05-config-e.toml");
        assert_tree_snippet_match(&fragments, treedir, "06-config-f.toml", "run/liboverdrop-rs/config.d-v0.1.0/06-config-f.toml");
        assert_tree_snippet_match(&fragments, treedir, "07-config-g.toml", "etc/liboverdrop-rs/config.d-v0.1.0/07-config-g.toml");
    }
}
