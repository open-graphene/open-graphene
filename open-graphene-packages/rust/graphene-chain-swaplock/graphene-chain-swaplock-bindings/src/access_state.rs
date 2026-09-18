use crate::generated::types::DataRoomAccessState;
use open_graphene_fc::{FcSerialize, FcSerializeError, sha256_bytes};

/// Hash a canonical room authority snapshot. Shared by native and WASM signers.
pub fn room_access_state_digest(state: &DataRoomAccessState) -> Result<[u8; 32], FcSerializeError> {
    if state.domain != "swaplock:data-room-access:v1" {
        return Err(invalid("unknown access snapshot domain"));
    }
    if state
        .members
        .windows(2)
        .any(|pair| pair[0].membership_instance >= pair[1].membership_instance)
    {
        return Err(invalid(
            "membership records must have strictly increasing instances",
        ));
    }
    Ok(sha256_bytes(&state.to_fc_bytes()?))
}

fn invalid(reason: &'static str) -> FcSerializeError {
    FcSerializeError::UnsupportedValue {
        type_name: "data_room_access_state",
        reason,
    }
}
