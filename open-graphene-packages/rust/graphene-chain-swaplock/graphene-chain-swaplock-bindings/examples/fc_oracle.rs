// JSON-lines oracle for cross-language regression vectors; never submits transactions.
use graphene_chain_swaplock_bindings::generated::{fc::FcSerialize, static_variants::Operation};
use std::io::{self, BufRead};
fn main() {
    for line in io::stdin().lock().lines() {
        let result = (|| -> Result<String, Box<dyn std::error::Error>> {
            let op: Operation = serde_json::from_str(&line?)?;
            Ok(op
                .to_fc_bytes()?
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>())
        })();
        match result {
            Ok(hex) => println!("{}", serde_json::json!({"hex":hex})),
            Err(e) => println!("{}", serde_json::json!({"error":e.to_string()})),
        }
    }
}
