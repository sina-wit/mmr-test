use alloy_primitives::B256;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct MerklizeProgramParams {
    pub leaves: Vec<B256>,
}
