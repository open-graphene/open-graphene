use thiserror::Error;

pub const SWAPLOCK_CHAIN_ID: &str =
    "2267f694d96b7ffdcba1a98c63c09e720a18a85ad34954e299c66d5a42234098";

#[derive(Debug, Error)]
pub enum SwaplockApiError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwaplockApi {
    chain_id: String,
}

impl SwaplockApi {
    pub fn mocked(chain_id: impl Into<String>) -> Self {
        Self {
            chain_id: chain_id.into(),
        }
    }

    pub async fn get_chain_id(&self) -> Result<String, SwaplockApiError> {
        Ok(self.chain_id.clone())
    }
}

impl Default for SwaplockApi {
    fn default() -> Self {
        Self::mocked(SWAPLOCK_CHAIN_ID)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_api_uses_swaplock_chain_id() {
        assert_eq!(SwaplockApi::default().chain_id, SWAPLOCK_CHAIN_ID);
    }
}
