//! Ergonomic builders for content cards: documents (or other content) stored off-chain,
//! identified by hash, always living inside a data room.
//!
//! Thin sugar over [`TransactionBuilder`](super::transaction::TransactionBuilder). The author must
//! be the room owner or a member with `DATA_ROOM_PERM_CREATE_CONTENT`; updating or removing
//! someone else's card takes `DATA_ROOM_PERM_MANAGE_CONTENT`.

use graphene_chain_swaplock_bindings::generated::ids::{
    AccountId, AssetId, ContentCardId, DataRoomId,
};
use graphene_chain_swaplock_bindings::generated::operations::{
    ContentCardCreateOperation, ContentCardRemoveOperation, ContentCardUpdateOperation,
};
use graphene_chain_swaplock_bindings::generated::static_variants::Operation;
use graphene_chain_swaplock_bindings::generated::types::Asset;
use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

use super::transaction::{PreparedTransaction, TransactionBuilder};

const CORE_ASSET_ID: &str = "1.3.0";

fn core_fee() -> Asset {
    Asset::new(0, AssetId(CORE_ASSET_ID.to_string()))
}

struct ContentCardCreateFields {
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
        Ok(Operation::content_card_create(ContentCardCreateOperation {
            fee: core_fee(),
            author: AccountId(self.author),
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
    new_url: Option<String>,
    new_type: Option<String>,
    new_description: Option<String>,
    new_content_key: Option<String>,
    new_storage_data: Option<String>,
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
            new_url: None,
            new_type: None,
            new_description: None,
            new_content_key: None,
            new_storage_data: None,
        }
    }

    /// New content hash, unique within the room.
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

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::content_card_update(ContentCardUpdateOperation {
            fee: core_fee(),
            caller: AccountId(self.caller),
            content_id: ContentCardId(self.content_id),
            new_hash: self.new_hash,
            new_url: self.new_url,
            new_type: self.new_type,
            new_description: self.new_description,
            new_content_key: self.new_content_key,
            new_storage_data: self.new_storage_data,
            extensions: vec![],
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
        }
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::content_card_remove(ContentCardRemoveOperation {
            fee: core_fee(),
            caller: AccountId(self.caller),
            content_id: ContentCardId(self.content_id),
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
    fn content_card_create_serializes_to_the_graphene_wire_shape() {
        let operation = fixture_fields().into_operation().unwrap();

        assert_eq!(
            serde_json::to_value(&operation).unwrap(),
            json!([85, {
                "fee": {"amount": 0, "asset_id": "1.3.0"},
                "author": "1.2.100",
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
            caller: AccountId("1.2.100".to_string()),
            content_id: ContentCardId("1.26.4".to_string()),
            new_hash: None,
            new_url: Some("ipfs://Qm456".to_string()),
            new_type: None,
            new_description: Some("v2".to_string()),
            new_content_key: None,
            new_storage_data: None,
            extensions: vec![],
        });

        assert_eq!(
            serde_json::to_value(&operation).unwrap(),
            json!([86, {
                "fee": {"amount": 0, "asset_id": "1.3.0"},
                "caller": "1.2.100",
                "content_id": "1.26.4",
                "new_hash": null,
                "new_url": "ipfs://Qm456",
                "new_type": null,
                "new_description": "v2",
                "new_content_key": null,
                "new_storage_data": null,
                "extensions": []
            }])
        );
    }

    #[test]
    fn content_card_remove_serializes_with_tag_eighty_seven() {
        let operation = Operation::content_card_remove(ContentCardRemoveOperation {
            fee: core_fee(),
            caller: AccountId("1.2.100".to_string()),
            content_id: ContentCardId("1.26.4".to_string()),
            extensions: vec![],
        });

        assert_eq!(
            serde_json::to_value(&operation).unwrap(),
            json!([87, {
                "fee": {"amount": 0, "asset_id": "1.3.0"},
                "caller": "1.2.100",
                "content_id": "1.26.4",
                "extensions": []
            }])
        );
    }
}
