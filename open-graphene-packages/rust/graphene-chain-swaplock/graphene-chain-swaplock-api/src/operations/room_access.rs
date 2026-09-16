use graphene_chain_swaplock_bindings::generated::{
    static_variants::Operation, types::DataRoomAccessState,
};
use open_graphene_core::ObjectId;

use crate::SwaplockApiError;

/// A signed condition bound to one room. It does not authorize a caller or fetch keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomAccessPrecondition {
    room: String,
    digest: String,
}

impl RoomAccessPrecondition {
    /// Accept a digest computed by an independent signer from a coherent snapshot.
    pub fn from_digest(
        room: impl Into<String>,
        digest: impl Into<String>,
    ) -> Result<Self, SwaplockApiError> {
        let room = room.into();
        ObjectId::parse(&room)?.require_type(1, 23)?;
        let digest = digest.into();
        if digest.len() != 64
            || !digest
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        {
            return Err(invalid("Expected a lowercase SHA-256 digest"));
        }
        Ok(Self { room, digest })
    }

    /// Hash the exact FC snapshot shared with the chain. Never silently reorder members.
    pub fn from_snapshot(state: &DataRoomAccessState) -> Result<Self, SwaplockApiError> {
        let digest = graphene_chain_swaplock_bindings::room_access_state_digest(state)
            .map_err(|_| invalid("Invalid access snapshot encoding or ordering"))?;
        Self::from_digest(state.room.0.clone(), hex::encode(digest))
    }

    pub fn room(&self) -> &str {
        &self.room
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }

    /// Attach this condition only to a membership/rotation operation for this room.
    /// Calling again with a different condition is rejected rather than overwriting intent.
    pub fn guard(&self, mut operation: Operation) -> Result<Operation, SwaplockApiError> {
        let (room, extensions) = match &mut operation {
            Operation::DataRoomMemberAddOperation(op) => (&op.room, &mut op.extensions),
            Operation::DataRoomMemberUpdateOperation(op) => (&op.room, &mut op.extensions),
            Operation::DataRoomMemberRemoveOperation(op) => (&op.room, &mut op.extensions),
            Operation::DataRoomRotateKeyOperation(op) => (&op.room, &mut op.extensions),
            _ => {
                return Err(invalid(
                    "Operation does not support a room access precondition",
                ));
            }
        };
        if room.0 != self.room {
            return Err(invalid("Access precondition belongs to another room"));
        }
        if extensions
            .expected_access_state
            .as_ref()
            .is_some_and(|value| value != &self.digest)
        {
            return Err(invalid(
                "Operation already has a different access precondition",
            ));
        }
        extensions.expected_access_state = Some(self.digest.clone());
        Ok(operation)
    }
}

fn invalid(message: &str) -> SwaplockApiError {
    SwaplockApiError::InvalidRoomAccess {
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{member_add_operation, member_remove_operation, rotate_key_operation};
    use graphene_chain_swaplock_bindings::generated::{
        ids::{AccountId, DataRoomId},
        static_variants::DataRoomMemberRef,
        types::DataRoomAccessMemberState,
    };

    fn state() -> DataRoomAccessState {
        DataRoomAccessState {
            domain: "swaplock:data-room-access:v1".into(),
            room: DataRoomId("1.23.7".into()),
            owner: AccountId("1.2.2".into()),
            encrypted: true,
            current_epoch: 3,
            members: vec![DataRoomAccessMemberState {
                membership_instance: 9,
                member: DataRoomMemberRef::AccountIdType(Box::new(AccountId("1.2.2".into()))),
                permissions: 255,
                member_key: "envelope".into(),
            }],
        }
    }
    #[test]
    fn computes_the_cpp_snapshot_fixture() {
        let condition = RoomAccessPrecondition::from_snapshot(&state()).unwrap();
        assert_eq!(
            condition.digest(),
            "3ef3e7a0f3b8dc2a2ea389b800623fa84c9dafe0ffde9f1c1e194e6516efcfbf"
        );
    }
    #[test]
    fn binds_condition_to_room_and_does_not_replace_existing_intent() {
        let condition = RoomAccessPrecondition::from_snapshot(&state()).unwrap();
        let op = member_add_operation(
            "1.2.2".into(),
            "1.23.7".into(),
            "1.2.3".into(),
            "sealed".into(),
            vec![],
            0,
        )
        .unwrap();
        let guarded = condition.guard(op).unwrap();
        assert_eq!(
            serde_json::to_value(&guarded).unwrap()[1]["extensions"]["expected_access_state"],
            condition.digest()
        );
        assert!(condition.guard(guarded.clone()).is_ok());
        let different = RoomAccessPrecondition::from_digest("1.23.7", "00".repeat(32)).unwrap();
        assert!(different.guard(guarded).is_err());
        let wrong_room =
            member_remove_operation("1.2.2".into(), "1.23.8".into(), "1.2.3".into()).unwrap();
        assert!(condition.guard(wrong_room).is_err());
    }
    #[test]
    fn rejects_malformed_domains_order_and_hashes() {
        let mut snapshot = state();
        snapshot.domain = "another-protocol".into();
        assert!(RoomAccessPrecondition::from_snapshot(&snapshot).is_err());
        snapshot = state();
        snapshot.members.push(snapshot.members[0].clone());
        assert!(RoomAccessPrecondition::from_snapshot(&snapshot).is_err());
        for value in ["ab".into(), "AB".repeat(32), "zz".repeat(32)] {
            assert!(RoomAccessPrecondition::from_digest("1.23.7", value).is_err());
        }
        assert!(RoomAccessPrecondition::from_digest("1.2.7", "ab".repeat(32)).is_err());
    }
    #[test]
    fn removal_and_rotation_use_different_snapshots() {
        let mut before = state();
        let mut reader = before.members[0].clone();
        reader.membership_instance = 10;
        reader.member = DataRoomMemberRef::AccountIdType(Box::new(AccountId("1.2.3".into())));
        reader.permissions = 0;
        before.members.push(reader);
        let mut after = before.clone();
        after.members.pop();
        let first = RoomAccessPrecondition::from_snapshot(&before).unwrap();
        let second = RoomAccessPrecondition::from_snapshot(&after).unwrap();
        assert_ne!(first.digest(), second.digest());
        let remove =
            member_remove_operation("1.2.2".into(), "1.23.7".into(), "1.2.3".into()).unwrap();
        let rotate = rotate_key_operation(
            "1.2.2".into(),
            "1.23.7".into(),
            vec![("1.2.2".into(), "new-envelope".into())],
        )
        .unwrap();
        assert!(first.guard(remove).is_ok());
        assert!(second.guard(rotate).is_ok());
    }
}
