use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::config::SourceConfig;
use crate::error::{Result, SpecGenError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSet {
    pub chain_repo: PathBuf,
    pub files: Vec<SourceFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    pub path: PathBuf,
    pub kind: SourceFileKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFileKind {
    Header,
}

pub fn discover_sources(config_path: &Path, source: &SourceConfig) -> Result<SourceSet> {
    let chain_repo = resolve_chain_repo(config_path, &source.chain_repo)?;
    if !chain_repo.is_dir() {
        return Err(SpecGenError::SourceRootMissing { path: chain_repo });
    }

    let mut files = Vec::new();
    for root in rpc_header_roots(&chain_repo) {
        if root.is_dir() {
            collect_headers(&root, &mut files)?;
        }
    }
    files.sort_by(|left, right| left.path.cmp(&right.path));
    files.dedup_by(|left, right| left.path == right.path);

    Ok(SourceSet { chain_repo, files })
}

pub fn resolve_chain_repo(config_path: &Path, chain_repo: &str) -> Result<PathBuf> {
    let path = PathBuf::from(chain_repo);
    if path.is_absolute() {
        return Ok(path);
    }

    let config_dir =
        config_path
            .parent()
            .ok_or_else(|| SpecGenError::SourcePathHasNoConfigParent {
                config_path: config_path.to_path_buf(),
                chain_repo: chain_repo.to_string(),
            })?;
    Ok(config_dir.join(path))
}

fn rpc_header_roots(chain_repo: &Path) -> [PathBuf; 4] {
    [
        chain_repo.join("libraries/app/include"),
        chain_repo.join("libraries/chain/include"),
        chain_repo.join("libraries/protocol/include"),
        chain_repo.join("libraries/plugins"),
    ]
}

fn collect_headers(root: &Path, files: &mut Vec<SourceFile>) -> Result<()> {
    for entry in WalkDir::new(root) {
        let entry = entry.map_err(|source| SpecGenError::DiscoverSources {
            path: root.to_path_buf(),
            source,
        })?;
        if entry.file_type().is_file() && entry.path().extension().is_some_and(|ext| ext == "hpp") {
            files.push(SourceFile {
                path: entry.path().to_path_buf(),
                kind: SourceFileKind::Header,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_relative_chain_repo_against_config_dir() {
        let resolved = resolve_chain_repo(
            Path::new(
                "/repo/open-graphene-bindings/graphene-bitshares/graphene-bitshares-spec/open-graphene.toml",
            ),
            "../../../blockchains/bitshares/bitshares-core",
        )
        .expect("resolve chain repo");

        assert_eq!(
            resolved,
            PathBuf::from(
                "/repo/open-graphene-bindings/graphene-bitshares/graphene-bitshares-spec/../../../blockchains/bitshares/bitshares-core"
            )
        );
    }
}
