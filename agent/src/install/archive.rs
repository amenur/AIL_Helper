//! Descompresión de .zip y .tar.gz. Ambos crates rechazan rutas que salgan
//! del directorio de destino (zip-slip).

use anyhow::{Result, bail};
use std::path::{Path, PathBuf};

pub fn extract(archive: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    let name = archive.to_string_lossy().to_lowercase();
    let file = std::fs::File::open(archive)?;
    if name.ends_with(".zip") {
        zip::ZipArchive::new(file)?.extract(dest)?;
    } else if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        tar::Archive::new(flate2::read::GzDecoder::new(file)).unpack(dest)?;
    } else {
        bail!("formato de archivo no soportado: {}", archive.display());
    }
    Ok(())
}

/// Busca un fichero por nombre dentro de un árbol (los zips de llama.cpp cambian de estructura).
pub fn find_file(root: &Path, file_name: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(root).ok()?;
    let mut dirs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            dirs.push(path);
        } else if path.file_name().is_some_and(|n| n == file_name) {
            return Some(path);
        }
    }
    dirs.into_iter().find_map(|d| find_file(&d, file_name))
}

#[cfg(unix)]
pub fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(perms.mode() | 0o755);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
pub fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
}
