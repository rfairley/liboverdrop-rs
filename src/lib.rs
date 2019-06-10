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

    pub fn scan_unique_files(
        &self,
        allow_hidden: bool,
        allowed_extensions: Option<&Vec<&str>>,
    ) -> failure::Fallible<collections::BTreeMap<String, path::PathBuf>> {
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

                // If hidden files not allowed, ignore dotfiles.
                if !allow_hidden && fname.starts_with('.') {
                    continue;
                };

                // If extensions are specified, proceed only if filename
                // has one of the allowed extensions.
                if let Some(allowed) = allowed_extensions {
                    if let Some(e) = fpath.extension() {
                        if !allowed.contains(&e.to_str().unwrap()) {
                            continue;
                        }
                    } else {
                        continue;
                    }
                }

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

    fn assert_fragments_relpath_match(
        fragments: &collections::BTreeMap<String, path::PathBuf>,
        root_dir: &str,
        filename: &str,
        relpath: &str,
    ) -> () {
        assert_eq!(
            fragments
                .get(&String::from(filename))
                .unwrap()
                .strip_prefix(root_dir)
                .unwrap(),
            &path::PathBuf::from(relpath)
        );
    }

    fn assert_fragments_hit(
        fragments: &collections::BTreeMap<String, path::PathBuf>,
        filename: &str,
    ) -> () {
        assert!(fragments.get(&String::from(filename)).is_some());
    }

    fn assert_fragments_miss(
        fragments: &collections::BTreeMap<String, path::PathBuf>,
        filename: &str,
    ) -> () {
        assert!(fragments.get(&String::from(filename)).is_none());
    }

    #[test]
    fn basic_override() {
        let treedir = "tests/fixtures/tree-basic";
        let dirs = vec![
            format!("{}/{}", treedir, "usr/lib"),
            format!("{}/{}", treedir, "run"),
            format!("{}/{}", treedir, "etc"),
        ];
        let allowed_extensions = vec![
            "toml",
        ];
        let expected_keys = vec![
            "01-config-a.toml".to_string(),
            "02-config-b.toml".to_string(),
            "03-config-c.toml".to_string(),
            "04-config-d.toml".to_string(),
            "05-config-e.toml".to_string(),
            "06-config-f.toml".to_string(),
            "07-config-g.toml".to_string(),
        ];

        let od_cfg = OverdropConf::new(&dirs, "liboverdrop-rs/config.d-v0.1.0");
        let fragments = od_cfg.scan_unique_files(false, Some(&allowed_extensions)).unwrap();

        assert_fragments_relpath_match(&fragments, treedir, "01-config-a.toml", "etc/liboverdrop-rs/config.d-v0.1.0/01-config-a.toml");
        assert_fragments_relpath_match(&fragments, treedir, "02-config-b.toml", "run/liboverdrop-rs/config.d-v0.1.0/02-config-b.toml");
        assert_fragments_relpath_match(&fragments, treedir, "03-config-c.toml", "etc/liboverdrop-rs/config.d-v0.1.0/03-config-c.toml");
        assert_fragments_relpath_match(&fragments, treedir, "04-config-d.toml", "usr/lib/liboverdrop-rs/config.d-v0.1.0/04-config-d.toml");
        assert_fragments_relpath_match(&fragments, treedir, "05-config-e.toml", "etc/liboverdrop-rs/config.d-v0.1.0/05-config-e.toml");
        assert_fragments_relpath_match(&fragments, treedir, "06-config-f.toml", "run/liboverdrop-rs/config.d-v0.1.0/06-config-f.toml");
        assert_fragments_relpath_match(&fragments, treedir, "07-config-g.toml", "etc/liboverdrop-rs/config.d-v0.1.0/07-config-g.toml");

        let fragments_keys: Vec<_> = fragments.keys().cloned().collect();
        assert_eq!(fragments_keys, expected_keys);
    }

    #[test]
    fn basic_override_new_full() {
        let treedir = "tests/fixtures/tree-basic";
        let dirs = vec![
            "usr/lib".to_string(),
            "run".to_string(),
            "etc".to_string(),
        ];
        let config_path = "liboverdrop-rs/config.d-";
        let version = Some("v0.1.0".to_string());
        let allowed_extensions = vec![
            "toml",
        ];
        let expected_keys = vec![
            "01-config-a.toml".to_string(),
            "02-config-b.toml".to_string(),
            "03-config-c.toml".to_string(),
            "04-config-d.toml".to_string(),
            "05-config-e.toml".to_string(),
            "06-config-f.toml".to_string(),
            "07-config-g.toml".to_string(),
        ];

        let od_cfg = OverdropConf::new_full(treedir, &dirs, config_path, version);
        let fragments = od_cfg.scan_unique_files(false, Some(&allowed_extensions)).unwrap();

        assert_fragments_relpath_match(&fragments, treedir, "01-config-a.toml", "etc/liboverdrop-rs/config.d-v0.1.0/01-config-a.toml");
        assert_fragments_relpath_match(&fragments, treedir, "02-config-b.toml", "run/liboverdrop-rs/config.d-v0.1.0/02-config-b.toml");
        assert_fragments_relpath_match(&fragments, treedir, "03-config-c.toml", "etc/liboverdrop-rs/config.d-v0.1.0/03-config-c.toml");
        assert_fragments_relpath_match(&fragments, treedir, "04-config-d.toml", "usr/lib/liboverdrop-rs/config.d-v0.1.0/04-config-d.toml");
        assert_fragments_relpath_match(&fragments, treedir, "05-config-e.toml", "etc/liboverdrop-rs/config.d-v0.1.0/05-config-e.toml");
        assert_fragments_relpath_match(&fragments, treedir, "06-config-f.toml", "run/liboverdrop-rs/config.d-v0.1.0/06-config-f.toml");
        assert_fragments_relpath_match(&fragments, treedir, "07-config-g.toml", "etc/liboverdrop-rs/config.d-v0.1.0/07-config-g.toml");

        let fragments_keys: Vec<_> = fragments.keys().cloned().collect();
        assert_eq!(fragments_keys, expected_keys);
    }

    #[test]
    fn basic_override_restrict_extensions() {
        let treedir = "tests/fixtures/tree-basic";
        let dirs = vec![
            format!("{}/{}", treedir, "etc"),
        ];
        let allowed_extensions = vec![
            "toml",
        ];

        let od_cfg = OverdropConf::new(&dirs, "liboverdrop-rs/config.d-v0.1.0");
        let fragments = od_cfg.scan_unique_files(false, Some(&allowed_extensions)).unwrap();

        assert_fragments_hit(&fragments, "01-config-a.toml");
        assert_fragments_miss(&fragments, "08-config-h.conf");
        assert_fragments_miss(&fragments, "noextension");
    }

    #[test]
    fn basic_override_allow_all_extensions() {
        let treedir = "tests/fixtures/tree-basic";
        let dirs = vec![
            format!("{}/{}", treedir, "etc"),
        ];

        let od_cfg = OverdropConf::new(&dirs, "liboverdrop-rs/config.d-v0.1.0");
        let fragments = od_cfg.scan_unique_files(false, None).unwrap();

        assert_fragments_hit(&fragments, "01-config-a.toml");
        assert_fragments_hit(&fragments, "config.conf");
        assert_fragments_hit(&fragments, "noextension");
    }

    #[test]
    fn basic_override_ignore_hidden() {
        let treedir = "tests/fixtures/tree-basic";
        let dirs = vec![
            format!("{}/{}", treedir, "etc"),
        ];

        let od_cfg = OverdropConf::new(&dirs, "liboverdrop-rs/config.d-v0.1.0");
        let fragments = od_cfg.scan_unique_files(false, None).unwrap();

        assert_fragments_hit(&fragments, "config.conf");
        assert_fragments_miss(&fragments, ".hidden.conf");
    }

    #[test]
    fn basic_override_allow_hidden() {
        let treedir = "tests/fixtures/tree-basic";
        let dirs = vec![
            format!("{}/{}", treedir, "etc"),
        ];

        let od_cfg = OverdropConf::new(&dirs, "liboverdrop-rs/config.d-v0.1.0");
        let fragments = od_cfg.scan_unique_files(true, None).unwrap();

        assert_fragments_hit(&fragments, "config.conf");
        assert_fragments_hit(&fragments, ".hidden.conf");
    }
}
