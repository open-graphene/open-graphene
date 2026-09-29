//! Ergonomic builders for content cards: documents (or other content) stored off-chain,
//! identified by hash, always living inside a data room.
//!
//! Thin sugar over [`TransactionBuilder`](super::transaction::TransactionBuilder). The author must
//! be the room owner or a member with `DATA_ROOM_PERM_CREATE_CONTENT`; updating or removing
//! someone else's card takes `DATA_ROOM_PERM_MANAGE_CONTENT`.

use graphene_chain_swaplock_bindings::generated::ids::{
    AccountId, AssetId, ContentCardGrantId, ContentCardId, ContentCardLinkId, DataRoomId,
};
use graphene_chain_swaplock_bindings::generated::operations::{
    ContentCardCreateOperation, ContentCardGrantCreateOperation, ContentCardGrantRevokeOperation,
    ContentCardLinkCreateOperation, ContentCardLinkRemoveOperation, ContentCardLinkUpdateOperation,
    ContentCardRemoveOperation, ContentCardUpdateOperation,
};
use graphene_chain_swaplock_bindings::generated::static_variants::DataRoomMemberRef;
use graphene_chain_swaplock_bindings::generated::static_variants::Operation;
use graphene_chain_swaplock_bindings::generated::types::Asset;
use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;
use crate::member::member_ref;

use super::transaction::{PreparedTransaction, TransactionBuilder};

const CORE_ASSET_ID: &str = "1.3.0";

fn core_fee() -> Asset {
    Asset::new(0, AssetId(CORE_ASSET_ID.to_string()))
}

/// Who pays for a card operation whose subject may be a bare key.
///
/// An account subject pays for itself unless told otherwise. A key cannot pay at all - the
/// chain's `fee_payer()` always returns an account - so it must be given a payer explicitly.
fn resolve_payer(
    subject: &DataRoomMemberRef,
    payer: Option<String>,
    operation: &'static str,
) -> Result<AccountId, SwaplockApiError> {
    if let Some(payer) = payer {
        return Ok(AccountId(payer));
    }
    match subject {
        DataRoomMemberRef::AccountIdType(account) => Ok((**account).clone()),
        // A key needs a payer; a ROOM cannot act at all - the chain rejects it in
        // validate(), so failing here is just the earlier, clearer error.
        DataRoomMemberRef::PublicKeyType(_) | DataRoomMemberRef::DataRoomIdType(_) => {
            Err(SwaplockApiError::KeyMemberNeedsPayer { operation })
        }
    }
}

struct ContentCardCreateFields {
    payer: Option<String>,
    author: String,
    room: String,
    hash: Option<String>,
    url: Option<String>,
    content_type: String,
    description: String,
    content_key: String,
    storage_data: Option<String>,
}

impl ContentCardCreateFields {
    fn into_operation(self) -> Result<Operation, SwaplockApiError> {
        let hash = self
            .hash
            .ok_or(SwaplockApiError::MissingTransferField { field: "hash" })?;
        let url = self
            .url
            .ok_or(SwaplockApiError::MissingTransferField { field: "url" })?;
        let storage_data = self
            .storage_data
            .ok_or(SwaplockApiError::MissingTransferField {
                field: "storage_data",
            })?;
        let author = member_ref(self.author);
        let payer = resolve_payer(&author, self.payer, "content_card_create")?;
        Ok(Operation::content_card_create(ContentCardCreateOperation {
            fee: core_fee(),
            payer,
            author,
            room: DataRoomId(self.room),
            hash,
            url,
            r#type: self.content_type,
            description: self.description,
            content_key: self.content_key,
            storage_data,
            extensions: vec![],
        }))
    }
}

/// Builder for `content_card_create`: publish a content card in a data room.
///
/// Required: the `author` (room owner or member with `DATA_ROOM_PERM_CREATE_CONTENT`), the `room`,
/// the content `.hash(..)` (unique per room), its `.url(..)` and the `.storage_data(..)` metadata.
/// Optional: `.content_type(..)`, `.description(..)` and `.content_key(..)` (leave unset for
/// public content).
pub struct ContentCardCreateRequest<'session> {
    session: &'session mut GrapheneSession,
    fields: ContentCardCreateFields,
}

impl<'session> ContentCardCreateRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        author: impl Into<String>,
        room: impl Into<String>,
    ) -> Self {
        Self {
            session,
            fields: ContentCardCreateFields {
                payer: None,
                author: author.into(),
                room: room.into(),
                hash: None,
                url: None,
                content_type: String::new(),
                description: String::new(),
                content_key: String::new(),
                storage_data: None,
            },
        }
    }

    /// The account paying the fee, when it is not the author.
    ///
    /// Required when the author is a bare public key: a key cannot pay. The payer only pays -
    /// authorship still rests on the author's own signature, so a payer cannot forge a card.
    pub fn payer(mut self, payer: impl Into<String>) -> Self {
        self.fields.payer = Some(payer.into());
        self
    }

    /// Content hash, unique within the room.
    pub fn hash(mut self, hash: impl Into<String>) -> Self {
        self.fields.hash = Some(hash.into());
        self
    }

    /// Where the content lives.
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.fields.url = Some(url.into());
        self
    }

    /// Content type / MIME type.
    pub fn content_type(mut self, content_type: impl Into<String>) -> Self {
        self.fields.content_type = content_type.into();
        self
    }

    /// Free-form description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.fields.description = description.into();
        self
    }

    /// Content encryption key material; leave unset for public content.
    pub fn content_key(mut self, content_key: impl Into<String>) -> Self {
        self.fields.content_key = content_key.into();
        self
    }

    /// Storage metadata.
    pub fn storage_data(mut self, storage_data: impl Into<String>) -> Self {
        self.fields.storage_data = Some(storage_data.into());
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = self.fields.into_operation()?;
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `content_card_update`: change a content card in place.
///
/// Required: the `caller` (the card's author, the room owner, or a member with
/// `DATA_ROOM_PERM_MANAGE_CONTENT`) and the `content_id`. Every field is a partial update; nothing
/// set means no change, which the node will reject, so set at least one.
pub struct ContentCardUpdateRequest<'session> {
    session: &'session mut GrapheneSession,
    caller: String,
    content_id: String,
    new_hash: Option<String>,
    expected_hash: Option<String>,
    new_url: Option<String>,
    new_type: Option<String>,
    new_description: Option<String>,
    new_content_key: Option<String>,
    new_storage_data: Option<String>,
    payer: Option<String>,
}

impl<'session> ContentCardUpdateRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        caller: impl Into<String>,
        content_id: impl Into<String>,
    ) -> Self {
        Self {
            session,
            caller: caller.into(),
            content_id: content_id.into(),
            new_hash: None,
            expected_hash: None,
            new_url: None,
            new_type: None,
            new_description: None,
            new_content_key: None,
            new_storage_data: None,
            payer: None,
        }
    }

    /// Require the current hash to match at execution time.
    pub fn expected_hash(mut self, hash: impl Into<String>) -> Self {
        self.expected_hash = Some(hash.into());
        self
    }

    /// New content hash.
    pub fn new_hash(mut self, hash: impl Into<String>) -> Self {
        self.new_hash = Some(hash.into());
        self
    }

    /// New content location.
    pub fn new_url(mut self, url: impl Into<String>) -> Self {
        self.new_url = Some(url.into());
        self
    }

    /// New content type / MIME type.
    pub fn new_type(mut self, content_type: impl Into<String>) -> Self {
        self.new_type = Some(content_type.into());
        self
    }

    /// New free-form description.
    pub fn new_description(mut self, description: impl Into<String>) -> Self {
        self.new_description = Some(description.into());
        self
    }

    /// New content key material; refreshes the card's key epoch.
    pub fn new_content_key(mut self, content_key: impl Into<String>) -> Self {
        self.new_content_key = Some(content_key.into());
        self
    }

    /// New storage metadata.
    pub fn new_storage_data(mut self, storage_data: impl Into<String>) -> Self {
        self.new_storage_data = Some(storage_data.into());
        self
    }

    /// The account paying the fee, when it is not the caller. Required for a key caller.
    pub fn payer(mut self, payer: impl Into<String>) -> Self {
        self.payer = Some(payer.into());
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let caller = member_ref(self.caller);
        let payer = resolve_payer(&caller, self.payer, "content_card_update")?;
        let operation = Operation::content_card_update(ContentCardUpdateOperation {
            fee: core_fee(),
            payer,
            caller,
            content_id: ContentCardId(self.content_id),
            new_hash: self.new_hash,
            new_url: self.new_url,
            new_type: self.new_type,
            new_description: self.new_description,
            new_content_key: self.new_content_key,
            new_storage_data: self.new_storage_data,
            extensions:
                graphene_chain_swaplock_bindings::generated::types::ContentCardUpdateOperationExt {
                    expected_hash: self.expected_hash,
                },
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `content_card_remove`: remove a content card.
///
/// Required: the `caller` (the card's author, the room owner, or a member with
/// `DATA_ROOM_PERM_MANAGE_CONTENT`) and the `content_id`.
pub struct ContentCardRemoveRequest<'session> {
    session: &'session mut GrapheneSession,
    caller: String,
    content_id: String,
    payer: Option<String>,
    expected_hash: Option<String>,
}

impl<'session> ContentCardRemoveRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        caller: impl Into<String>,
        content_id: impl Into<String>,
    ) -> Self {
        Self {
            session,
            caller: caller.into(),
            content_id: content_id.into(),
            payer: None,
            expected_hash: None,
        }
    }

    /// The account paying the fee, when it is not the caller. Required for a key caller.
    pub fn payer(mut self, payer: impl Into<String>) -> Self {
        self.payer = Some(payer.into());
        self
    }

    pub fn expected_hash(mut self, hash: impl Into<String>) -> Self {
        self.expected_hash = Some(hash.into());
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let caller = member_ref(self.caller);
        let payer = resolve_payer(&caller, self.payer, "content_card_remove")?;
        let operation = Operation::content_card_remove(ContentCardRemoveOperation {
            fee: core_fee(),
            payer,
            caller,
            content_id: ContentCardId(self.content_id),
            extensions:
                graphene_chain_swaplock_bindings::generated::types::ContentCardRemoveOperationExt {
                    expected_hash: self.expected_hash,
                },
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Build a bare `content_card_grant_create` operation — for wrapping in a proposal or
/// composing into a larger transaction.
pub fn content_card_grant_create_operation(
    granter: String,
    content_id: String,
    grantee: String,
    key: String,
) -> Result<Operation, SwaplockApiError> {
    if key.is_empty() {
        return Err(SwaplockApiError::MissingTransferField { field: "key" });
    }
    Ok(Operation::content_card_grant_create(
        ContentCardGrantCreateOperation {
            fee: core_fee(),
            granter: AccountId(granter),
            content_id: ContentCardId(content_id),
            grantee: member_ref(grantee),
            key,
            extensions: vec![],
        },
    ))
}

/// Builder for `content_card_grant_create`: hand ONE card to ONE recipient outside room
/// membership — the accountable form of disclosure.
///
/// Required: the `granter` (room owner or member with `DATA_ROOM_PERM_GRANT_CONTENT`), the
/// `content_id`, the `.grantee(..)` (an account or a bare public key) and the `.key(..)` — the
/// card's content key encrypted to the grantee. Grants survive key rotation and disappear with
/// their card.
pub struct ContentCardGrantCreateRequest<'session> {
    session: &'session mut GrapheneSession,
    granter: String,
    content_id: String,
    grantee: Option<String>,
    key: Option<String>,
}

impl<'session> ContentCardGrantCreateRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        granter: impl Into<String>,
        content_id: impl Into<String>,
    ) -> Self {
        Self {
            session,
            granter: granter.into(),
            content_id: content_id.into(),
            grantee: None,
            key: None,
        }
    }

    /// The recipient: an account (name or id) or a bare public key.
    pub fn grantee(mut self, grantee: impl Into<String>) -> Self {
        self.grantee = Some(grantee.into());
        self
    }

    /// The card's content key encrypted to the grantee.
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let grantee = self
            .grantee
            .ok_or(SwaplockApiError::MissingTransferField { field: "grantee" })?;
        let key = self
            .key
            .ok_or(SwaplockApiError::MissingTransferField { field: "key" })?;
        let operation =
            content_card_grant_create_operation(self.granter, self.content_id, grantee, key)?;
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `content_card_grant_revoke`: retract a grant.
///
/// Required: the `caller` (the original granter, the room owner, or a member with
/// `DATA_ROOM_PERM_GRANT_CONTENT`) and the `grant_id`. Revocation removes the on-chain record;
/// it cannot remove what the grantee has already read.
pub struct ContentCardGrantRevokeRequest<'session> {
    session: &'session mut GrapheneSession,
    caller: String,
    grant_id: String,
}

impl<'session> ContentCardGrantRevokeRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        caller: impl Into<String>,
        grant_id: impl Into<String>,
    ) -> Self {
        Self {
            session,
            caller: caller.into(),
            grant_id: grant_id.into(),
        }
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::content_card_grant_revoke(ContentCardGrantRevokeOperation {
            fee: core_fee(),
            caller: AccountId(self.caller),
            grant_id: ContentCardGrantId(self.grant_id),
            extensions: vec![],
        });
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

    fn fixture_fields() -> ContentCardCreateFields {
        ContentCardCreateFields {
            payer: None,
            author: "1.2.100".to_string(),
            room: "1.23.7".to_string(),
            hash: Some("abc123".to_string()),
            url: Some("ipfs://Qm123".to_string()),
            content_type: "application/pdf".to_string(),
            description: "term sheet".to_string(),
            content_key: "ENC_CONTENT".to_string(),
            storage_data: Some("{\"pin\":true}".to_string()),
        }
    }

    #[test]
    fn expected_hash_extension_matches_native_fc_vector() {
        use graphene_chain_swaplock_bindings::generated::types::ContentCardUpdateOperationExt;
        use open_graphene_fc::FcSerialize;
        let mut bytes = Vec::new();
        ContentCardUpdateOperationExt {
            expected_hash: None,
        }
        .fc_serialize(&mut bytes)
        .unwrap();
        assert_eq!(bytes, vec![0]);
        bytes.clear();
        ContentCardUpdateOperationExt {
            expected_hash: Some("revision-1".into()),
        }
        .fc_serialize(&mut bytes)
        .unwrap();
        assert_eq!(bytes, b"\x01\x00\x0arevision-1");
    }

    #[test]
    fn content_card_create_serializes_to_the_graphene_wire_shape() {
        let operation = fixture_fields().into_operation().unwrap();

        assert_eq!(
            serde_json::to_value(&operation).unwrap(),
            json!([85, {
                "fee": {"amount": 0, "asset_id": "1.3.0"},
                "payer": "1.2.100",
                "author": [0, "1.2.100"],
                "room": "1.23.7",
                "hash": "abc123",
                "url": "ipfs://Qm123",
                "type": "application/pdf",
                "description": "term sheet",
                "content_key": "ENC_CONTENT",
                "storage_data": "{\"pin\":true}",
                "extensions": []
            }])
        );
    }

    #[test]
    fn content_card_create_requires_hash_url_and_storage_data() {
        let mut missing_hash = fixture_fields();
        missing_hash.hash = None;
        assert_eq!(
            missing_hash.into_operation().unwrap_err().to_string(),
            "missing transfer field `hash`"
        );

        let mut missing_url = fixture_fields();
        missing_url.url = None;
        assert_eq!(
            missing_url.into_operation().unwrap_err().to_string(),
            "missing transfer field `url`"
        );

        let mut missing_storage = fixture_fields();
        missing_storage.storage_data = None;
        assert_eq!(
            missing_storage.into_operation().unwrap_err().to_string(),
            "missing transfer field `storage_data`"
        );
    }

    #[test]
    fn content_card_update_serializes_partial_fields_with_tag_eighty_six() {
        let operation = Operation::content_card_update(ContentCardUpdateOperation {
            fee: core_fee(),
            payer: AccountId("1.2.100".to_string()),
            caller: member_ref("1.2.100"),
            content_id: ContentCardId("1.26.4".to_string()),
            new_hash: None,
            new_url: Some("ipfs://Qm456".to_string()),
            new_type: None,
            new_description: Some("v2".to_string()),
            new_content_key: None,
            new_storage_data: None,
            extensions:
                graphene_chain_swaplock_bindings::generated::types::ContentCardUpdateOperationExt {
                    expected_hash: None,
                },
        });

        assert_eq!(
            serde_json::to_value(&operation).unwrap(),
            json!([86, {
                "fee": {"amount": 0, "asset_id": "1.3.0"},
                "payer": "1.2.100",
                "caller": [0, "1.2.100"],
                "content_id": "1.26.4",
                "new_url": "ipfs://Qm456",
                "new_description": "v2",
                "extensions": {}
            }])
        );
    }

    #[test]
    fn content_card_remove_serializes_with_tag_eighty_seven() {
        let operation = Operation::content_card_remove(ContentCardRemoveOperation {
            fee: core_fee(),
            payer: AccountId("1.2.100".to_string()),
            caller: member_ref("1.2.100"),
            content_id: ContentCardId("1.26.4".to_string()),
            extensions:
                graphene_chain_swaplock_bindings::generated::types::ContentCardRemoveOperationExt {
                    expected_hash: None,
                },
        });

        assert_eq!(
            serde_json::to_value(&operation).unwrap(),
            json!([87, {
                "fee": {"amount": 0, "asset_id": "1.3.0"},
                "payer": "1.2.100",
                "caller": [0, "1.2.100"],
                "content_id": "1.26.4",
                "extensions": {}
            }])
        );
    }
}

#[cfg(test)]
mod grant_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn grant_create_serializes_with_tag_ninety() {
        let operation = content_card_grant_create_operation(
            "1.2.100".to_string(),
            "1.26.7".to_string(),
            "1.2.9".to_string(),
            "DEK_FOR_9".to_string(),
        )
        .unwrap();

        assert_eq!(
            serde_json::to_value(&operation).unwrap(),
            json!([90, {
                "fee": {"amount": 0, "asset_id": "1.3.0"},
                "granter": "1.2.100",
                "content_id": "1.26.7",
                "grantee": [0, "1.2.9"],
                "key": "DEK_FOR_9",
                "extensions": []
            }])
        );
    }

    #[test]
    fn grant_create_accepts_a_bare_key_grantee() {
        const KEY: &str = "BTS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV";
        let operation = content_card_grant_create_operation(
            "1.2.100".to_string(),
            "1.26.7".to_string(),
            KEY.to_string(),
            "DEK_FOR_POD".to_string(),
        )
        .unwrap();

        assert_eq!(
            serde_json::to_value(&operation).unwrap()[1]["grantee"],
            json!([1, KEY])
        );
    }

    #[test]
    fn grant_create_requires_the_key() {
        let error = content_card_grant_create_operation(
            "1.2.100".to_string(),
            "1.26.7".to_string(),
            "1.2.9".to_string(),
            String::new(),
        )
        .unwrap_err();
        assert_eq!(error.to_string(), "missing transfer field `key`");
    }
}

/// Builder for `content_card_link_create`: embed a card into another room by reference.
///
/// Required: the `caller` (a member of the card's HOME room, holding
/// `DATA_ROOM_PERM_CREATE_CONTENT` in the target room), the `content_id` and the target
/// `room`. An encrypted card additionally requires `.link_key(..)` - its content key wrapped
/// to the target room's key.
pub struct ContentCardLinkCreateRequest<'session> {
    session: &'session mut GrapheneSession,
    caller: String,
    content_id: String,
    room: String,
    link_key: String,
    payer: Option<String>,
}

impl<'session> ContentCardLinkCreateRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        caller: impl Into<String>,
        content_id: impl Into<String>,
        room: impl Into<String>,
    ) -> Self {
        Self {
            session,
            caller: caller.into(),
            content_id: content_id.into(),
            room: room.into(),
            link_key: String::new(),
            payer: None,
        }
    }

    /// The card's content key wrapped to the TARGET room's key. Required for an encrypted
    /// card; must stay empty for a public one.
    pub fn link_key(mut self, link_key: impl Into<String>) -> Self {
        self.link_key = link_key.into();
        self
    }

    /// The account paying the fee, when it is not the caller. Required for a key caller.
    pub fn payer(mut self, payer: impl Into<String>) -> Self {
        self.payer = Some(payer.into());
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let caller = member_ref(self.caller);
        let payer = resolve_payer(&caller, self.payer, "content_card_link_create")?;
        let operation = Operation::content_card_link_create(ContentCardLinkCreateOperation {
            fee: core_fee(),
            payer,
            caller,
            content_id: ContentCardId(self.content_id),
            room: DataRoomId(self.room),
            link_key: self.link_key,
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `content_card_link_update`: re-wrap a link after either side rotates.
///
/// Required: the `caller` (the linker or a member of the card's home room), the `link_id`
/// and the `new_link_key`. The chain restamps which epochs the fresh wrap corresponds to.
pub struct ContentCardLinkUpdateRequest<'session> {
    session: &'session mut GrapheneSession,
    caller: String,
    link_id: String,
    new_link_key: Option<String>,
    payer: Option<String>,
}

impl<'session> ContentCardLinkUpdateRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        caller: impl Into<String>,
        link_id: impl Into<String>,
    ) -> Self {
        Self {
            session,
            caller: caller.into(),
            link_id: link_id.into(),
            new_link_key: None,
            payer: None,
        }
    }

    /// The card's content key re-wrapped to the target room's key.
    pub fn new_link_key(mut self, new_link_key: impl Into<String>) -> Self {
        self.new_link_key = Some(new_link_key.into());
        self
    }

    /// The account paying the fee, when it is not the caller. Required for a key caller.
    pub fn payer(mut self, payer: impl Into<String>) -> Self {
        self.payer = Some(payer.into());
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let new_link_key = self
            .new_link_key
            .ok_or(SwaplockApiError::MissingTransferField {
                field: "new_link_key",
            })?;
        let caller = member_ref(self.caller);
        let payer = resolve_payer(&caller, self.payer, "content_card_link_update")?;
        let operation = Operation::content_card_link_update(ContentCardLinkUpdateOperation {
            fee: core_fee(),
            payer,
            caller,
            link_id: ContentCardLinkId(self.link_id),
            new_link_key,
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `content_card_link_remove`: take a card off a room's table.
///
/// Required: the `caller` (the linker, the home room via `MANAGE_CONTENT`, or the target
/// room via `MANAGE_CONTENT`) and the `link_id`. The card itself is untouched.
pub struct ContentCardLinkRemoveRequest<'session> {
    session: &'session mut GrapheneSession,
    caller: String,
    link_id: String,
    payer: Option<String>,
}

impl<'session> ContentCardLinkRemoveRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        caller: impl Into<String>,
        link_id: impl Into<String>,
    ) -> Self {
        Self {
            session,
            caller: caller.into(),
            link_id: link_id.into(),
            payer: None,
        }
    }

    /// The account paying the fee, when it is not the caller. Required for a key caller.
    pub fn payer(mut self, payer: impl Into<String>) -> Self {
        self.payer = Some(payer.into());
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let caller = member_ref(self.caller);
        let payer = resolve_payer(&caller, self.payer, "content_card_link_remove")?;
        let operation = Operation::content_card_link_remove(ContentCardLinkRemoveOperation {
            fee: core_fee(),
            payer,
            caller,
            link_id: ContentCardLinkId(self.link_id),
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}
