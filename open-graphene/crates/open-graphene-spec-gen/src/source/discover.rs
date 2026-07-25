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
    /// A .cpp translation unit. Chain objects are often reflected here
    /// (FC_REFLECT_DERIVED_NO_TYPENAME in libraries/chain/*.cpp), so these
    /// files are scanned for reflect macros only.
    Source,
}

pub fn discover_sources(config_path: &Path, source: &SourceConfig) -> Result<SourceSet> {
    let configured = source
        .chain_repo_env
        .as_deref()
        .and_then(|name| std::env::var(name).ok())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| source.chain_repo.clone());
    let chain_repo = resolve_chain_repo(config_path, &configured)?;
    if !chain_repo.is_dir() {
        return Err(SpecGenError::SourceRootMissing { path: chain_repo });
    }

    let mut files = Vec::new();
    for root in rpc_header_roots(&chain_repo) {
        if root.is_dir() {
            collect_headers(&root, &mut files)?;
        }
    }
    for root in reflect_source_roots(&chain_repo) {
        if root.is_dir() {
            collect_sources(&root, &mut files)?;
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

/// Translation-unit roots scanned for reflect macros. Chain objects are
/// registered via FC_REFLECT_DERIVED_NO_TYPENAME in .cpp files; without
/// these the affected structs silently fall back to declaration order.
fn reflect_source_roots(chain_repo: &Path) -> [PathBuf; 4] {
    [
        chain_repo.join("libraries/app"),
        chain_repo.join("libraries/chain"),
        chain_repo.join("libraries/protocol"),
        chain_repo.join("libraries/plugins"),
    ]
}

fn collect_headers(root: &Path, files: &mut Vec<SourceFile>) -> Result<()> {
    collect_by_extension(root, files, "hpp", SourceFileKind::Header)
}

fn collect_sources(root: &Path, files: &mut Vec<SourceFile>) -> Result<()> {
    collect_by_extension(root, files, "cpp", SourceFileKind::Source)
}

fn collect_by_extension(
    root: &Path,
    files: &mut Vec<SourceFile>,
    extension: &str,
    kind: SourceFileKind,
) -> Result<()> {
    for entry in WalkDir::new(root) {
        let entry = entry.map_err(|source| SpecGenError::DiscoverSources {
            path: root.to_path_buf(),
            source,
        })?;
        if entry.file_type().is_file()
            && entry.path().extension().is_some_and(|ext| ext == extension)
        {
            files.push(SourceFile {
                path: entry.path().to_path_buf(),
                kind,
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
                "/repo/open-graphene-packages/rust/graphene-chain-bitshares/graphene-chain-bitshares-spec/open-graphene.toml",
            ),
            "../../../../blockchains/bitshares/bitshares-core",
        )
        .expect("resolve chain repo");

        assert_eq!(
            resolved,
            PathBuf::from(
                "/repo/open-graphene-packages/rust/graphene-chain-bitshares/graphene-chain-bitshares-spec/../../../../blockchains/bitshares/bitshares-core"
            )
        );
    }
}
