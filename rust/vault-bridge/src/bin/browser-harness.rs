//! Test-only JSON-lines transport; no socket and no persistent user vault.
use std::io::{self, BufRead, Write};
use vault_bridge::{development::TemporaryVault, Commands, Request};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut service = TemporaryVault::new()?;
    for line in io::stdin().lock().lines() {
        let line = zeroize::Zeroizing::new(line?);
        let response = match serde_json::from_str::<Request>(&line) {
            Ok(request) => serde_json::to_value(service.dispatch(request))?,
            Err(_) => {
                serde_json::json!({"Err":{"code":"invalid_request","message":"Invalid request."}})
            }
        };
        let text = zeroize::Zeroizing::new(serde_json::to_string(&response)?);
        writeln!(io::stdout(), "{}", text.as_str())?;
        io::stdout().flush()?;
    }
    Ok(())
}
