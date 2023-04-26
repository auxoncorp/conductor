use crate::envsub::{envsub, EnvSubError};
use std::{fs, path::PathBuf};

const RESC_PATH_PREFIX: char = '@';

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, thiserror::Error)]
pub enum PlatformDescriptionError {
    #[error("The platform description is empty")]
    Emtpy,
    #[error("Could not determine a file name for local platform description file '{_0:?}'")]
    FileName(PathBuf),
    #[error(
        "Encountered an IO error while reading the local platform description file '{_0:?}'. {_1}"
    )]
    Io(PathBuf, String),
    #[error(transparent)]
    EnvSub(#[from] EnvSubError),
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum PlatformDescription {
    /// A platform 'repl' file provided by Renode.
    /// Can begin with a '@' character to bypass the auto detection.
    ProvidedByRenode(PathBuf),
    /// A local 'repl' file, relative to the config (or absolute).
    /// Supports environment substitution in the file path and content.
    /// The data contains the file name and content.
    LocalFile(String, String),
    // TODO - add support for string literals, do the unindent/indent logic, etc
    // for now we're just required either renode-provided or local repl files
    // A string containing a platform description.
    // Supports environment substitution in the content.
    //String(String),
}

impl PlatformDescription {
    pub fn from_config(cfg: &str) -> Result<Self, PlatformDescriptionError> {
        let trimmed_cfg = cfg.trim();
        if trimmed_cfg.is_empty() {
            return Err(PlatformDescriptionError::Emtpy);
        }

        let envsubd_cfg = envsub(trimmed_cfg)?;
        let is_renode_provided = trimmed_cfg.starts_with(RESC_PATH_PREFIX);

        let p = if is_renode_provided {
            PathBuf::from(trimmed_cfg.trim_start_matches(RESC_PATH_PREFIX))
        } else {
            PathBuf::from(&envsubd_cfg)
        };

        // TODO - define a better heuristic for this
        if !p.exists() || is_renode_provided {
            // Assume renode-provided, they look like relative paths, both don't exist
            // because they're relative to the renode installation directory
            Ok(PlatformDescription::ProvidedByRenode(p))
        } else {
            let file_name = p
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| PlatformDescriptionError::FileName(p.clone()))?;
            let content = envsub(
                &fs::read_to_string(&p)
                    .map_err(|e| PlatformDescriptionError::Io(p.clone(), e.to_string()))?,
            )?;

            Ok(PlatformDescription::LocalFile(
                file_name.to_owned(),
                content,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    const REPL_CONTENT_RAW: &str = indoc! {r#"
        phy2: Network.EthernetPhysicalLayer @ ethernet 2
            Id1: ${CARGO_PKG_NAME}

        phy3: Network.EthernetPhysicalLayer @ ethernet 3
            Id1: ${FOOBAR_NOT_SET:-123}
    "#};

    const REPL_CONTENT_SUBD: &str = indoc! {r#"
        phy2: Network.EthernetPhysicalLayer @ ethernet 2
            Id1: conductor

        phy3: Network.EthernetPhysicalLayer @ ethernet 3
            Id1: 123
    "#};

    #[test]
    fn plat_desc_happy_path() {
        assert_eq!(
            PlatformDescription::from_config("platforms/boards/stm32f4_discovery-kit.repl"),
            Ok(PlatformDescription::ProvidedByRenode(PathBuf::from(
                "platforms/boards/stm32f4_discovery-kit.repl"
            )))
        );

        assert_eq!(
            PlatformDescription::from_config("@platforms/boards/stm32f4_discovery-kit.repl"),
            Ok(PlatformDescription::ProvidedByRenode(PathBuf::from(
                "platforms/boards/stm32f4_discovery-kit.repl"
            )))
        );

        let out = tempfile::tempdir().unwrap();
        let pkg_name = env!("CARGO_PKG_NAME");
        let local_repl_path =
            PathBuf::from(format!("{}/{pkg_name}/myplat.repl", out.path().display()));
        fs::create_dir_all(local_repl_path.parent().unwrap()).unwrap();
        fs::write(local_repl_path, REPL_CONTENT_RAW).unwrap();

        assert_eq!(
            PlatformDescription::from_config(&format!(
                "{}/${{CARGO_PKG_NAME}}/myplat.repl",
                out.path().display()
            )),
            Ok(PlatformDescription::LocalFile(
                "myplat.repl".to_owned(),
                REPL_CONTENT_SUBD.to_owned()
            ))
        );
    }
}
