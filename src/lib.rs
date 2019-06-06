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
        base_dirs: &[path::PathBuf],
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

    // TODO: implement this to allow base_dirs in a different root (root_dir), and
    // versioning option like in https://github.com/overdrop/overdrop-sebool/blob/master/src/od_cfg.rs#L9.
    // pub fn new(
    //     root_dir: path::PathBuf,
    //     base_dirs: &[path::PathBuf],
    //     config_path: &str,
    //     version: Option<String>,
    // ) -> Self {

    // }

    // TODO: add options to exclude/include prefix/file suffix or glob (like in https://github.com/overdrop/overdrop-sebool/blob/master/src/od_cfg.rs#L42)

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
        prefix: &String,
        filename: &str,
        relpath: &str,
    ) -> () {
        assert_eq!(
            fragments
                .get(&String::from(filename))
                .unwrap()
                .strip_prefix(prefix.as_str())
                .unwrap(),
            &path::PathBuf::from(relpath)
        );
    }

    #[test]
    fn basic_override() {
        let treedir = String::from("tests/fixtures/tree-basic");
        let dirs = vec![
            path::PathBuf::from(format!("{}/{}", treedir, "usr/lib")),
            path::PathBuf::from(format!("{}/{}", treedir, "run")),
            path::PathBuf::from(format!("{}/{}", treedir, "etc")),
        ];
        let od_cfg = OverdropConf::new(&dirs, "name/config.d");
        let fragments = od_cfg.scan_unique_files().unwrap();

        assert_tree_snippet_match(&fragments, &treedir, "01-config-a.toml", "etc/name/config.d/01-config-a.toml");
        assert_tree_snippet_match(&fragments, &treedir, "02-config-b.toml", "run/name/config.d/02-config-b.toml");
        assert_tree_snippet_match(&fragments, &treedir, "03-config-c.toml", "etc/name/config.d/03-config-c.toml");
        assert_tree_snippet_match(&fragments, &treedir, "04-config-d.toml", "usr/lib/name/config.d/04-config-d.toml");
        assert_tree_snippet_match(&fragments, &treedir, "05-config-e.toml", "etc/name/config.d/05-config-e.toml");
        assert_tree_snippet_match(&fragments, &treedir, "06-config-f.toml", "run/name/config.d/06-config-f.toml");
        assert_tree_snippet_match(&fragments, &treedir, "07-config-g.toml", "etc/name/config.d/07-config-g.toml");
    }
}
