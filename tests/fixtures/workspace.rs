use std::path::{Path, PathBuf};

/// Test workspace with a temporary root directory.
#[derive(Debug)]
pub struct WorkspaceFixture {
    root: PathBuf,
}

impl WorkspaceFixture {
    /// Create a new workspace in the system temp directory.
    pub fn new() -> std::io::Result<Self> {
        let root = std::env::temp_dir().join(format!("forge-workspace-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    pub fn with_sample_files() -> std::io::Result<Self> {
        let fixture = Self::new()?;
        fixture.seed_files(&[
            ("notes/todo.txt", "alpha\nbeta\ngamma\n"),
            ("docs/guide.md", "# Guide\nHello world\n"),
            ("src/main.rs", "fn main() {}\n"),
        ])?;
        Ok(fixture)
    }

    /// Return the workspace root path.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Join a relative path against the workspace root.
    pub fn path(&self, rel: impl AsRef<Path>) -> PathBuf {
        self.root.join(rel)
    }

    /// Write a file relative to the workspace root.
    pub fn write_file(
        &self,
        rel: impl AsRef<Path>,
        contents: impl AsRef<[u8]>,
    ) -> std::io::Result<PathBuf> {
        let path = self.path(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, contents)?;
        Ok(path)
    }

    /// Seed the workspace with a small file tree.
    pub fn seed_files(&self, files: &[(&str, &str)]) -> std::io::Result<()> {
        for (path, contents) in files {
            self.write_file(path, contents)?;
        }
        Ok(())
    }
}

impl Drop for WorkspaceFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
