use std::fs;
use std::path::{Path, PathBuf};

use open_graphene_json_schema::Protocol;

use crate::error::{Result, SpecGenError};

pub fn write_protocol_json(config_path: &Path, dist: &str, protocol: &Protocol) -> Result<PathBuf> {
    let output_path = resolve_dist_path(config_path, dist)?;
    let output_dir = output_path
        .parent()
        .ok_or_else(|| SpecGenError::OutputPathHasNoParent {
            config_path: config_path.to_path_buf(),
            dist: dist.to_string(),
        })?;

    fs::create_dir_all(output_dir).map_err(|source| SpecGenError::CreateOutputDir {
        path: output_dir.to_path_buf(),
        source,
    })?;

    let json = serde_json::to_string_pretty(protocol).map_err(|source| {
        SpecGenError::SerializeProtocol {
            config_path: config_path.to_path_buf(),
            source,
        }
    })?;

    fs::write(&output_path, format!("{json}\n")).map_err(|source| SpecGenError::WriteOutput {
        path: output_path.clone(),
        source,
    })?;

    Ok(output_path)
}

pub fn resolve_dist_path(config_path: &Path, dist: &str) -> Result<PathBuf> {
    let dist_path = PathBuf::from(dist);
    if dist_path.is_absolute() {
        return Ok(dist_path);
    }

    let config_dir = config_path
        .parent()
        .ok_or_else(|| SpecGenError::OutputPathHasNoParent {
            config_path: config_path.to_path_buf(),
            dist: dist.to_string(),
        })?;
    Ok(config_dir.join(dist_path))
}
