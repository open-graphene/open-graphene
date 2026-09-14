//! Ergonomic builders for data rooms: shared containers of content cards that can be attached to
//! a tokenized asset or an account, with per-member encrypted keys and permission flags.
//!
//! Thin sugar over [`TransactionBuilder`](super::transaction::TransactionBuilder). Omit the room
//! key at creation for a public room; encryption can be bootstrapped later with
//! `data_room_rotate_key`.

use graphene_chain_swaplock_bindings::generated::ids::{
    AccountId, AssetId, DataRoomId, PUBLIC_KEY_PREFIX,
};
use graphene_chain_swaplock_bindings::generated::operations::{
    DataRoomCreateOperation, DataRoomDeleteOperation, DataRoomMemberAddOperation,
    DataRoomMemberRemoveOperation, DataRoomMemberUpdateOperation, DataRoomRotateKeyOperation,
    DataRoomUpdateOperation,
};
use graphene_chain_swaplock_bindings::generated::static_variants::{
    DataRoomMemberRef, DataRoomSubject, Operation,
};
use graphene_chain_swaplock_bindings::generated::types::Asset;
use open_graphene_core::ObjectId;
use open_graphene_fc::decode_public_key;
use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;
use crate::member::{is_public_key, member_ref};

use super::transaction::{PreparedTransaction, TransactionBuilder};

const CORE_ASSET_ID: &str = "1.3.0";

fn core_fee() -> Asset {
    Asset::new(0, AssetId(CORE_ASSET_ID.to_string()))
}

/// Member may update the room name / description.
pub const DATA_ROOM_PERM_UPDATE_ROOM: u32 = 0x0001;
/// Member may add new members.
pub const DATA_ROOM_PERM_ADD_MEMBERS: u32 = 0x0002;
/// Member may remove members.
pub const DATA_ROOM_PERM_REMOVE_MEMBERS: u32 = 0x0004;
/// Member may change other members' permission flags.
pub const DATA_ROOM_PERM_MANAGE_PERMISSIONS: u32 = 0x0008;
/// Member may create content cards in the room.
pub const DATA_ROOM_PERM_CREATE_CONTENT: u32 = 0x0010;
/// Member may update / remove other members' content cards.
pub const DATA_ROOM_PERM_MANAGE_CONTENT: u32 = 0x0020;
/// Member may rotate the room encryption key.
pub const DATA_ROOM_PERM_ROTATE_KEYS: u32 = 0x0040;
/// Member may grant single content cards to people outside the room.
pub const DATA_ROOM_PERM_GRANT_CONTENT: u32 = 0x0080;
/// All permission flags combined.
///
/// This is a MIRROR of `DATA_ROOM_PERM_ALL` in the protocol's `data_room.hpp`, and the
/// validation below refuses anything outside it — so a flag the node has learned and this
/// constant has not is refused HERE, before the transaction is ever built. That is the
/// failure mode worth naming: it does not look like a stale mirror, it looks like the chain
/// rejecting a right the interface just offered.
pub const DATA_ROOM_PERM_ALL: u32 = 0x00FF;

/// Reject permission flags the chain does not know about (mirrors the node's validation).
fn validate_permissions(permissions: u32) -> Result<(), SwaplockApiError> {
    if permissions & !DATA_ROOM_PERM_ALL != 0 {
        return Err(SwaplockApiError::InvalidTransfer {
            message: format!("unknown data room permission flags: {permissions:#x}"),
        });
    }
    Ok(())
}

/// Sort epoch keys ascending by epoch, as the chain's flat_map wire format requires; reject a
/// duplicate epoch.
fn sorted_epoch_keys(
    mut epoch_keys: Vec<(u32, String)>,
) -> Result<Vec<(u32, String)>, SwaplockApiError> {
    epoch_keys.sort_by_key(|(epoch, _)| *epoch);
    for pair in epoch_keys.windows(2) {
        if pair[0].0 == pair[1].0 {
            return Err(SwaplockApiError::InvalidTransfer {
                message: format!("duplicate epoch key for epoch {}", pair[1].0),
            });
        }
    }
    Ok(epoch_keys)
}

/// Sort order of a member reference, matching the chain's `operator<` for
/// `data_room_member_ref`: variant tag first, then value.
///
/// Accounts order by object id instance. Keys order by their **raw 33 bytes**, which is what
/// `public_key_type::operator<` compares - not by the base58 text, whose ordering differs. Get
/// this wrong and the flat_map goes out on the wire unsorted.
fn member_sort_key(member: &str) -> Result<(u8, u64, [u8; 33]), SwaplockApiError> {
    if is_public_key(member) {
        let bytes = decode_public_key(member, Some(PUBLIC_KEY_PREFIX)).map_err(|error| {
            SwaplockApiError::InvalidTransfer {
                message: format!("invalid member public key `{member}`: {error}"),
            }
        })?;
        Ok((1, 0, bytes))
    } else {
        let instance = ObjectId::parse(member)?.require_type(1, 2)?.instance;
        Ok((0, instance, [0u8; 33]))
    }
}

/// Sort member keys as the chain's flat_map wire format requires; reject a malformed member or
/// a duplicate one. Members may be accounts or bare public keys.
fn sorted_member_keys(
    member_keys: Vec<(String, String)>,
) -> Result<Vec<(DataRoomMemberRef, String)>, SwaplockApiError> {
    let mut keyed = member_keys
        .into_iter()
        .map(|(member, key)| Ok((member_sort_key(&member)?, member, key)))
        .collect::<Result<Vec<_>, SwaplockApiError>>()?;
    keyed.sort_by_key(|entry| entry.0);
    for pair in keyed.windows(2) {
        if pair[0].0 == pair[1].0 {
            return Err(SwaplockApiError::InvalidTransfer {
                message: format!("duplicate member key for member `{}`", pair[1].1),
            });
        }
    }
    Ok(keyed
        .into_iter()
        .map(|(_, member, key)| (member_ref(member), key))
        .collect())
}

/// Build a bare `data_room_member_add` operation — for wrapping in a proposal. The fluent
/// request uses the same constructor, so wire shape and validation stay identical.
pub fn member_add_operation(
    caller: String,
    room: String,
    account: String,
    member_key: String,
    epoch_keys: Vec<(u32, String)>,
    permissions: u32,
) -> Result<Operation, SwaplockApiError> {
    validate_permissions(permissions)?;
    Ok(Operation::data_room_member_add(
        DataRoomMemberAddOperation {
            fee: core_fee(),
            caller: AccountId(caller),
            room: DataRoomId(room),
            member: member_ref(account),
            member_key,
            epoch_keys: sorted_epoch_keys(epoch_keys)?,
            permissions,
            extensions: vec![],
        },
    ))
}

/// Build a bare `data_room_member_remove` operation — for composing into a larger
/// transaction (the remove-and-rotate flow) or wrapping in a proposal.
pub fn member_remove_operation(
    caller: String,
    room: String,
    member: String,
) -> Result<Operation, SwaplockApiError> {
    Ok(Operation::data_room_member_remove(
        DataRoomMemberRemoveOperation {
            fee: core_fee(),
            caller: AccountId(caller),
            room: DataRoomId(room),
            member: member_ref(member),
            extensions: vec![],
        },
    ))
}

/// Build a bare `data_room_rotate_key` operation — for wrapping in a proposal.
///
/// The owner's envelope rides in `member_keys` like everyone else's: the owner is a member,
/// and the chain asserts full coverage of the member set.
pub fn rotate_key_operation(
    caller: String,
    room: String,
    member_keys: Vec<(String, String)>,
) -> Result<Operation, SwaplockApiError> {
    if member_keys.is_empty() {
        return Err(SwaplockApiError::MissingTransferField {
            field: "member_keys",
        });
    }
    Ok(Operation::data_room_rotate_key(
        DataRoomRotateKeyOperation {
            fee: core_fee(),
            caller: AccountId(caller),
            room: DataRoomId(room),
            member_keys: sorted_member_keys(member_keys)?,
            extensions: vec![],
        },
    ))
}

/// Builder for `data_room_create`: open a new data room owned by `owner`.
///
/// Required: the `owner` and the room `name` (unique per owner). Attach the room to a tokenized
/// asset or an account with `.subject_asset(..)` / `.subject_account(..)` (defaults to no
/// subject); attaching to an asset requires the owner to be its issuer. Set `.room_key(..)` with
/// the room key encrypted to the owner for an encrypted room; omit it for a public room.
pub struct DataRoomCreateRequest<'session> {
    session: &'session mut GrapheneSession,
    owner: String,
    name: String,
    description: String,
    subject: DataRoomSubject,
    room_key: Option<String>,
    write_policy: Option<u32>,
}

impl<'session> DataRoomCreateRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        owner: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Self {
            session,
            owner: owner.into(),
            name: name.into(),
            description: String::new(),
            subject: DataRoomSubject::VoidT(Box::new(open_graphene_fc::VoidT)),
            room_key: None,
            write_policy: None,
        }
    }

    /// Free-form room description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Attach the room to a tokenized asset (the owner must be the asset's issuer).
    pub fn subject_asset(mut self, asset: impl Into<String>) -> Self {
        self.subject = DataRoomSubject::AssetIdType(Box::new(AssetId(asset.into())));
        self
    }

    /// Attach the room to an account.
    pub fn subject_account(mut self, account: impl Into<String>) -> Self {
        self.subject = DataRoomSubject::AccountIdType(Box::new(AccountId(account.into())));
        self
    }

    /// The room key encrypted to the owner; omit for a public room.
    pub fn room_key(mut self, room_key: impl Into<String>) -> Self {
        self.room_key = Some(room_key.into());
        self
    }

    /// Require current editors and compare-and-swap for this room permanently.
    pub fn strict_writes(mut self) -> Self {
        self.write_policy = Some(1);
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::data_room_create(DataRoomCreateOperation {
            fee: core_fee(),
            owner: AccountId(self.owner),
            name: self.name,
            description: self.description,
            subject: self.subject,
            room_key: self.room_key,
            extensions: graphene_chain_swaplock_bindings::generated::types::DataRoomCreateOperationExt { write_policy: self.write_policy },
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `data_room_update`: change a data room in place.
///
/// Required: the `caller` and the `room` id. Every field is a partial update; nothing set means no
/// change, which the node will reject, so set at least one. Name and description need the owner or
/// a member with [`DATA_ROOM_PERM_UPDATE_ROOM`]; changing the subject or transferring ownership is
/// owner-only.
pub struct DataRoomUpdateRequest<'session> {
    session: &'session mut GrapheneSession,
    caller: String,
    room: String,
    new_name: Option<String>,
    new_description: Option<String>,
    new_subject: Option<DataRoomSubject>,
    new_owner: Option<String>,
}

impl<'session> DataRoomUpdateRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        caller: impl Into<String>,
        room: impl Into<String>,
    ) -> Self {
        Self {
            session,
            caller: caller.into(),
            room: room.into(),
            new_name: None,
            new_description: None,
            new_subject: None,
            new_owner: None,
        }
    }

    /// New room name (unique per owner).
    pub fn new_name(mut self, name: impl Into<String>) -> Self {
        self.new_name = Some(name.into());
        self
    }

    /// New free-form description.
    pub fn new_description(mut self, description: impl Into<String>) -> Self {
        self.new_description = Some(description.into());
        self
    }

    /// Detach the room from its subject (owner only).
    pub fn new_subject_none(mut self) -> Self {
        self.new_subject = Some(DataRoomSubject::VoidT(Box::new(open_graphene_fc::VoidT)));
        self
    }

    /// Attach the room to a tokenized asset (owner only).
    pub fn new_subject_asset(mut self, asset: impl Into<String>) -> Self {
        self.new_subject = Some(DataRoomSubject::AssetIdType(Box::new(AssetId(
            asset.into(),
        ))));
        self
    }

    /// Attach the room to an account (owner only).
    pub fn new_subject_account(mut self, account: impl Into<String>) -> Self {
        self.new_subject = Some(DataRoomSubject::AccountIdType(Box::new(AccountId(
            account.into(),
        ))));
        self
    }

    /// Transfer room ownership to another account (owner only).
    pub fn new_owner(mut self, owner: impl Into<String>) -> Self {
        self.new_owner = Some(owner.into());
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::data_room_update(DataRoomUpdateOperation {
            fee: core_fee(),
            caller: AccountId(self.caller),
            room: DataRoomId(self.room),
            new_name: self.new_name,
            new_description: self.new_description,
            new_subject: self.new_subject,
            new_owner: self.new_owner.map(AccountId),
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `data_room_delete`: remove a room, cascade-removing its content cards, members and
/// key epoch records. Owner only.
pub struct DataRoomDeleteRequest<'session> {
    session: &'session mut GrapheneSession,
    caller: String,
    room: String,
}

impl<'session> DataRoomDeleteRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        caller: impl Into<String>,
        room: impl Into<String>,
    ) -> Self {
        Self {
            session,
            caller: caller.into(),
            room: room.into(),
        }
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::data_room_delete(DataRoomDeleteOperation {
            fee: core_fee(),
            caller: AccountId(self.caller),
            room: DataRoomId(self.room),
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `data_room_member_add`: add `account` as a member of `room`.
///
/// Required: the `caller` (owner or member with [`DATA_ROOM_PERM_ADD_MEMBERS`]), the `room` and
/// the `account`. For encrypted rooms set `.member_key(..)` with the room key encrypted to the new
/// member and add an `.epoch_key(..)` per historical epoch (e.g. when re-adding a former member).
/// Grant abilities with `.permissions(..)` (defaults to none; see the `DATA_ROOM_PERM_*` flags).
pub struct DataRoomMemberAddRequest<'session> {
    session: &'session mut GrapheneSession,
    caller: String,
    room: String,
    account: String,
    member_key: String,
    epoch_keys: Vec<(u32, String)>,
    permissions: u32,
}

impl<'session> DataRoomMemberAddRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        caller: impl Into<String>,
        room: impl Into<String>,
        account: impl Into<String>,
    ) -> Self {
        Self {
            session,
            caller: caller.into(),
            room: room.into(),
            account: account.into(),
            member_key: String::new(),
            epoch_keys: vec![],
            permissions: 0,
        }
    }

    /// The current room key encrypted to the new member; leave unset for a public room.
    pub fn member_key(mut self, member_key: impl Into<String>) -> Self {
        self.member_key = member_key.into();
        self
    }

    /// Add the room key of a historical `epoch`, encrypted to the member. Call once per epoch.
    pub fn epoch_key(mut self, epoch: u32, key: impl Into<String>) -> Self {
        self.epoch_keys.push((epoch, key.into()));
        self
    }

    /// Permission flags for the member, a bitwise OR of the `DATA_ROOM_PERM_*` constants.
    pub fn permissions(mut self, permissions: u32) -> Self {
        self.permissions = permissions;
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = member_add_operation(
            self.caller,
            self.room,
            self.account,
            self.member_key,
            self.epoch_keys,
            self.permissions,
        )?;
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `data_room_member_update`: replace a member's permission flags.
///
/// Required: the `caller` (owner or member with [`DATA_ROOM_PERM_MANAGE_PERMISSIONS`]), the
/// `room`, the member `account` and the new `permissions`. The owner's member record cannot be
/// modified.
pub struct DataRoomMemberUpdateRequest<'session> {
    session: &'session mut GrapheneSession,
    caller: String,
    room: String,
    account: String,
    permissions: u32,
}

impl<'session> DataRoomMemberUpdateRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        caller: impl Into<String>,
        room: impl Into<String>,
        account: impl Into<String>,
        permissions: u32,
    ) -> Self {
        Self {
            session,
            caller: caller.into(),
            room: room.into(),
            account: account.into(),
            permissions,
        }
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        validate_permissions(self.permissions)?;
        let operation = Operation::data_room_member_update(DataRoomMemberUpdateOperation {
            fee: core_fee(),
            caller: AccountId(self.caller),
            room: DataRoomId(self.room),
            member: member_ref(self.account),
            permissions: self.permissions,
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `data_room_member_remove`: remove a member from a room.
///
/// Required: the `caller` (owner or member with [`DATA_ROOM_PERM_REMOVE_MEMBERS`]), the `room` and
/// the member `account`. The owner cannot be removed; the member's key epoch history is retained
/// so previously encrypted content stays decryptable for remaining members.
pub struct DataRoomMemberRemoveRequest<'session> {
    session: &'session mut GrapheneSession,
    caller: String,
    room: String,
    account: String,
}

impl<'session> DataRoomMemberRemoveRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        caller: impl Into<String>,
        room: impl Into<String>,
        account: impl Into<String>,
    ) -> Self {
        Self {
            session,
            caller: caller.into(),
            room: room.into(),
            account: account.into(),
        }
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::data_room_member_remove(DataRoomMemberRemoveOperation {
            fee: core_fee(),
            caller: AccountId(self.caller),
            room: DataRoomId(self.room),
            member: member_ref(self.account),
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `data_room_rotate_key`: rotate a room's encryption key, starting a new key epoch.
///
/// Required: the `caller` (owner or member with [`DATA_ROOM_PERM_ROTATE_KEYS`]), the `room`, and
/// a `.member_key(..)` per member - the owner included, since the owner is a member and the
/// chain requires the set to cover every member, no strangers. Also bootstraps encryption on a
/// previously public room.
pub struct DataRoomRotateKeyRequest<'session> {
    session: &'session mut GrapheneSession,
    caller: String,
    room: String,
    member_keys: Vec<(String, String)>,
}

impl<'session> DataRoomRotateKeyRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        caller: impl Into<String>,
        room: impl Into<String>,
    ) -> Self {
        Self {
            session,
            caller: caller.into(),
            room: room.into(),
            member_keys: vec![],
        }
    }

    /// Add the new room key encrypted to a member `account`. Call once per member.
    pub fn member_key(mut self, account: impl Into<String>, key: impl Into<String>) -> Self {
        self.member_keys.push((account.into(), key.into()));
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = rotate_key_operation(self.caller, self.room, self.member_keys)?;
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn data_room_create_serializes_to_the_graphene_wire_shape() {
        let operation = Operation::data_room_create(DataRoomCreateOperation {
            fee: core_fee(),
            owner: AccountId("1.2.100".to_string()),
            name: "deal-room".to_string(),
            description: "diligence".to_string(),
            subject: DataRoomSubject::AssetIdType(Box::new(AssetId("1.3.5".to_string()))),
            room_key: Some("ENC_OWNER".to_string()),
            extensions: graphene_chain_swaplock_bindings::generated::types::DataRoomCreateOperationExt { write_policy: None },
        });

        assert_eq!(
            serde_json::to_value(&operation).unwrap(),
            json!([78, {
                "fee": {"amount": 0, "asset_id": "1.3.0"},
                "owner": "1.2.100",
                "name": "deal-room",
                "description": "diligence",
                "subject": [1, "1.3.5"],
                "room_key": "ENC_OWNER",
                "extensions": {}
            }])
        );
    }

    /// A `void_t` subject must go out as `[0, {}]`, never `[0, null]`.
    ///
    /// This asserted `null` until 2026-07-25 — which is what serialising `()` produces, and
    /// what a node rejects with `Bad Cast: Invalid cast from type 'null_type' to Object`,
    /// making the builder's own default unusable. See `open_graphene_fc::VoidT`.
    #[test]
    fn data_room_create_defaults_to_a_public_room_without_subject() {
        let operation = Operation::data_room_create(DataRoomCreateOperation {
            fee: core_fee(),
            owner: AccountId("1.2.100".to_string()),
            name: "deal-room".to_string(),
            description: String::new(),
            subject: DataRoomSubject::VoidT(Box::new(open_graphene_fc::VoidT)),
            room_key: None,
            extensions: graphene_chain_swaplock_bindings::generated::types::DataRoomCreateOperationExt { write_policy: None },
        });

        assert_eq!(
            serde_json::to_value(&operation).unwrap(),
            json!([78, {
                "fee": {"amount": 0, "asset_id": "1.3.0"},
                "owner": "1.2.100",
                "name": "deal-room",
                "description": "",
                "subject": [0, {}],
                "extensions": {}
            }])
        );
    }

    #[test]
    fn member_add_sorts_epoch_keys_and_serializes_with_tag_eighty_one() {
        let operation = member_add_operation(
            "1.2.100".to_string(),
            "1.23.7".to_string(),
            "1.2.101".to_string(),
            "ENC_MEMBER".to_string(),
            vec![(3, "K3".to_string()), (1, "K1".to_string())],
            DATA_ROOM_PERM_CREATE_CONTENT | DATA_ROOM_PERM_UPDATE_ROOM,
        )
        .unwrap();

        assert_eq!(
            serde_json::to_value(&operation).unwrap(),
            json!([81, {
                "fee": {"amount": 0, "asset_id": "1.3.0"},
                "caller": "1.2.100",
                "room": "1.23.7",
                "member": [0, "1.2.101"],
                "member_key": "ENC_MEMBER",
                "epoch_keys": [[1, "K1"], [3, "K3"]],
                "permissions": 17,
                "extensions": []
            }])
        );
    }

    #[test]
    fn member_add_rejects_a_duplicate_epoch_key() {
        let error = member_add_operation(
            "1.2.100".to_string(),
            "1.23.7".to_string(),
            "1.2.101".to_string(),
            String::new(),
            vec![(2, "A".to_string()), (2, "B".to_string())],
            0,
        )
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "invalid transfer: duplicate epoch key for epoch 2"
        );
    }

    #[test]
    fn member_add_rejects_unknown_permission_flags() {
        let error = member_add_operation(
            "1.2.100".to_string(),
            "1.23.7".to_string(),
            "1.2.101".to_string(),
            String::new(),
            vec![],
            0x0100,
        )
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "invalid transfer: unknown data room permission flags: 0x100"
        );
    }

    #[test]
    fn rotate_key_sorts_member_keys_by_account_instance() {
        let operation = rotate_key_operation(
            "1.2.100".to_string(),
            "1.23.7".to_string(),
            vec![
                ("1.2.10".to_string(), "K10".to_string()),
                ("1.2.9".to_string(), "K9".to_string()),
            ],
        )
        .unwrap();

        assert_eq!(
            serde_json::to_value(&operation).unwrap(),
            json!([84, {
                "fee": {"amount": 0, "asset_id": "1.3.0"},
                "caller": "1.2.100",
                "room": "1.23.7",
                "member_keys": [[[0, "1.2.9"], "K9"], [[0, "1.2.10"], "K10"]],
                "extensions": []
            }])
        );
    }

    /// The chain orders `data_room_member_ref` by variant tag first, then by value - and for a
    /// key, "value" means its raw 33 bytes, which `public_key_type::operator<` compares. Sorting
    /// keys by their base58 text instead would put the flat_map on the wire out of order.
    #[test]
    fn rotate_key_orders_accounts_before_keys_and_keys_by_raw_bytes() {
        const KEY_A: &str = "BTS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV";
        const KEY_B: &str = "BTS7jDPoMwyjVH5obFmqzFNp4Ffp7G2nvC7FKFkrMBpo7Sy4uq5Mj";

        let operation = rotate_key_operation(
            "1.2.100".to_string(),
            "1.23.7".to_string(),
            vec![
                (KEY_B.to_string(), "KB".to_string()),
                ("1.2.10".to_string(), "K10".to_string()),
                (KEY_A.to_string(), "KA".to_string()),
                ("1.2.9".to_string(), "K9".to_string()),
            ],
        )
        .unwrap();

        // What the chain's comparator would produce, derived rather than hand-written: accounts
        // first, keys after, and the keys ordered by their decoded bytes.
        let mut keys = [KEY_A, KEY_B];
        keys.sort_by_key(|key| decode_public_key(key, Some(PUBLIC_KEY_PREFIX)).unwrap());
        let key_values: Vec<&str> = keys
            .iter()
            .map(|key| if *key == KEY_A { "KA" } else { "KB" })
            .collect();

        assert_eq!(
            serde_json::to_value(&operation).unwrap(),
            json!([84, {
                "fee": {"amount": 0, "asset_id": "1.3.0"},
                "caller": "1.2.100",
                "room": "1.23.7",
                "member_keys": [
                    [[0, "1.2.9"], "K9"],
                    [[0, "1.2.10"], "K10"],
                    [[1, keys[0]], key_values[0]],
                    [[1, keys[1]], key_values[1]],
                ],
                "extensions": []
            }])
        );
    }

    /// The same key cannot appear twice, exactly as for an account.
    #[test]
    fn rotate_key_rejects_a_duplicate_key_member() {
        const KEY_A: &str = "BTS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV";

        let duplicate = rotate_key_operation(
            "1.2.100".to_string(),
            "1.23.7".to_string(),
            vec![
                (KEY_A.to_string(), "KA".to_string()),
                (KEY_A.to_string(), "KA-again".to_string()),
            ],
        );

        assert_eq!(
            duplicate.unwrap_err().to_string(),
            format!("invalid transfer: duplicate member key for member `{KEY_A}`")
        );
    }

    #[test]
    fn rotate_key_requires_member_keys() {
        let missing_members =
            rotate_key_operation("1.2.100".to_string(), "1.23.7".to_string(), vec![]).unwrap_err();
        assert_eq!(
            missing_members.to_string(),
            "missing transfer field `member_keys`"
        );
    }

    #[test]
    fn rotate_key_rejects_a_duplicate_or_malformed_member_account() {
        let duplicate = rotate_key_operation(
            "1.2.100".to_string(),
            "1.23.7".to_string(),
            vec![
                ("1.2.9".to_string(), "A".to_string()),
                ("1.2.9".to_string(), "B".to_string()),
            ],
        )
        .unwrap_err();
        assert_eq!(
            duplicate.to_string(),
            "invalid transfer: duplicate member key for member `1.2.9`"
        );

        let malformed = rotate_key_operation(
            "1.2.100".to_string(),
            "1.23.7".to_string(),
            vec![("1.2.bogus".to_string(), "A".to_string())],
        )
        .unwrap_err();
        assert!(matches!(malformed, SwaplockApiError::ObjectId(_)));

        let not_an_account = rotate_key_operation(
            "1.2.100".to_string(),
            "1.23.7".to_string(),
            vec![("1.3.9".to_string(), "A".to_string())],
        )
        .unwrap_err();
        assert!(matches!(not_an_account, SwaplockApiError::ObjectId(_)));
    }

    /// Maska i lista flag muszą się zgadzać ZE SOBĄ.
    ///
    /// Czego ten test NIE sprawdza i co go raz ominęło: zgodności z protokołem.
    /// Gdy do `data_room.hpp` doszło `GRANT_CONTENT`, tutaj nie doszło nic —
    /// lista i maska dalej zgadzały się wzajemnie, więc test milczał, a klient
    /// odrzucał prawo, które łańcuch znał. Wartość `0x00FF` niżej jest tu
    /// przepisana Z PROTOKOŁU celowo: to jedyne miejsce, w którym rozjazd
    /// z nim w ogóle może się zapalić.
    #[test]
    fn permission_flags_cover_the_chain_mask() {
        assert_eq!(
            DATA_ROOM_PERM_UPDATE_ROOM
                | DATA_ROOM_PERM_ADD_MEMBERS
                | DATA_ROOM_PERM_REMOVE_MEMBERS
                | DATA_ROOM_PERM_MANAGE_PERMISSIONS
                | DATA_ROOM_PERM_CREATE_CONTENT
                | DATA_ROOM_PERM_MANAGE_CONTENT
                | DATA_ROOM_PERM_ROTATE_KEYS
                | DATA_ROOM_PERM_GRANT_CONTENT,
            DATA_ROOM_PERM_ALL
        );
        assert_eq!(
            DATA_ROOM_PERM_ALL, 0x00FF,
            "musi zgadzać się z data_room.hpp"
        );
    }
}
