//! Digest transakcji swaplock, liczony po stronie klienta.
//!
//! # Po co to istnieje
//!
//! Kto podpisuje transakcję, dostaje od budowniczego `sha256(chain_id ‖ tx)` —
//! liczbę, której **nie da się odróżnić od żadnego innego skrótu**. Bez
//! własnej serializacji podpisujący ufa budowniczemu, że podsunięta transakcja
//! jest tą, którą człowiekowi pokazano.
//!
//! Ten krateks to zamyka: bierze transakcję i identyfikator łańcucha, składa
//! bajty tak samo jak węzeł, i oddaje digest do PORÓWNANIA. Rozjazd znaczy, że
//! budowniczy podsuwa co innego, niż mówi.
//!
//! # Dlaczego tutaj, a nie w warstwie kluczy
//!
//! To jest wiedza o PROTOKOLE — czym jest transakcja i jak się serializuje —
//! a nie o kluczach. Warstwa kluczy zna role, WIF i kanoniczność podpisu;
//! gdyby nosiła jeszcze układ bajtów operacji, każda zmiana protokołu
//! przelewałaby się do niej bez powodu. Tu za to przyda się każdemu, kto chce
//! sprawdzić transakcję — także temu, kto nie trzyma żadnego skarbca.

use graphene_chain_swaplock_bindings::generated::{FcSerialize, Transaction};
use wasm_bindgen::prelude::*;

/// Bajty transakcji w kodowaniu FC — dokładnie te, które hashuje węzeł.
///
/// `transaction` to JSON w kształcie, jaki oddaje budowniczy.
#[wasm_bindgen(js_name = serializeTransaction)]
pub fn serialize_transaction(transaction: JsValue) -> Result<Vec<u8>, JsValue> {
    let json: serde_json::Value = serde_wasm_bindgen::from_value(transaction)
        .map_err(|e| JsValue::from_str(&format!("nie JSON transakcji: {e}")))?;
    let tx: Transaction = serde_json::from_value(json)
        .map_err(|e| JsValue::from_str(&format!("nie transakcja swaplock: {e}")))?;

    let mut out = Vec::new();
    tx.fc_serialize(&mut out)
        .map_err(|e| JsValue::from_str(&format!("serializacja: {e}")))?;
    Ok(out)
}

/// `sha256(chain_id ‖ transakcja)` — to, co podpisuje klucz.
///
/// `chain_id` w hex, tak jak oddaje go węzeł.
#[wasm_bindgen(js_name = transactionDigest)]
pub fn transaction_digest(chain_id: &str, transaction: JsValue) -> Result<Vec<u8>, JsValue> {
    let chain = hex::decode(chain_id.trim())
        .map_err(|_| JsValue::from_str("chain_id musi być w hex"))?;
    let bytes = serialize_transaction(transaction)?;

    let mut buf = Vec::with_capacity(chain.len() + bytes.len());
    buf.extend_from_slice(&chain);
    buf.extend_from_slice(&bytes);
    Ok(graphene_chain_swaplock_bindings::generated::sha256_bytes(&buf).to_vec())
}

/// Wygodne porównanie: czy digest budowniczego zgadza się z policzonym tutaj.
///
/// Wydzielone, żeby konsument nie musiał sam pisać porównania w stałym czasie
/// ani martwić się o wielkość liter w hex.
#[wasm_bindgen(js_name = digestMatches)]
pub fn digest_matches(
    chain_id: &str,
    transaction: JsValue,
    claimed_digest_hex: &str,
) -> Result<bool, JsValue> {
    let ours = transaction_digest(chain_id, transaction)?;
    let theirs = hex::decode(claimed_digest_hex.trim())
        .map_err(|_| JsValue::from_str("digest musi być w hex"))?;
    Ok(ours.len() == theirs.len()
        && ours
            .iter()
            .zip(theirs.iter())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0)
}
