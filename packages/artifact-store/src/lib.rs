pub trait ArtifactStore {
    fn put(&self, key: &str, bytes: &[u8]) -> Result<(), ArtifactStoreError>;
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, ArtifactStoreError>;
}

#[derive(Debug)]
pub struct ArtifactStoreError {
    pub message: String,
}
