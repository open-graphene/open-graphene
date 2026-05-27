use graphene_chain_swaplock_bindings::generated::OperationHistoryObject;

#[derive(Debug, Clone, PartialEq)]
pub struct AccountHistoryPage {
    items: Vec<OperationHistoryObject>,
    limit: u32,
    offset: u32,
    next_offset: Option<u32>,
}

impl AccountHistoryPage {
    pub(super) fn new(
        items: Vec<OperationHistoryObject>,
        limit: u32,
        offset: u32,
        next_offset: Option<u32>,
    ) -> Self {
        Self {
            items,
            limit,
            offset,
            next_offset,
        }
    }

    pub fn items(&self) -> &[OperationHistoryObject] {
        &self.items
    }

    pub fn limit(&self) -> u32 {
        self.limit
    }

    pub fn offset(&self) -> u32 {
        self.offset
    }

    pub fn next_offset(&self) -> Option<u32> {
        self.next_offset
    }

    pub fn has_more(&self) -> bool {
        self.next_offset.is_some()
    }

    pub fn into_items(self) -> Vec<OperationHistoryObject> {
        self.items
    }
}
