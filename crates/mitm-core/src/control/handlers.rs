use serde_json::{json, Value};

use crate::MitmCore;

pub async fn call(core: &MitmCore, method: &str, params: &Value) -> Result<Value, String> {
    match method {
        "status" => Ok(status(core).await),
        "register_container" => {
            core.register_container(&text(params, "id")?, &text(params, "name")?)?;
            Ok(json!({}))
        }
        "remove_container" => {
            core.remove_container(&text(params, "id")?)?;
            Ok(json!({}))
        }
        "ca_info" => ca_info(core),
        "ca_pem" => Ok(json!({ "pem": core.ca_pem()? })),
        "list_containers" => Ok(json!(core.list_containers())),
        "start_run" => Ok(json!({ "run_id": core.start_container_run(&text(params, "id")?)? })),
        "delete_flows" => {
            let run_id = params.get("run_id").and_then(Value::as_i64);
            Ok(json!({ "deleted": core.delete_container_flows(&text(params, "id")?, run_id)? }))
        }
        other => Err(format!("Unknown method '{other}'")),
    }
}

fn text(params: &Value, key: &str) -> Result<String, String> {
    params
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("Missing '{key}'"))
}

async fn status(core: &MitmCore) -> Value {
    let listeners: Vec<Value> = core
        .listener_status()
        .await
        .into_iter()
        .map(|l| json!({ "label": l.config.label, "address": l.config.address, "running": l.running }))
        .collect();
    json!({
        "version": env!("CARGO_PKG_VERSION"),
        "token": core.container_proxy_token(),
        "listeners": listeners,
    })
}

fn ca_info(core: &MitmCore) -> Result<Value, String> {
    let info = core.ca_info()?;
    Ok(json!({
        "pem": core.ca_pem()?,
        "fingerprint": info.fingerprint,
        "common_name": info.common_name,
        "not_before": info.not_before,
        "not_after": info.not_after,
        "scope": if info.scope == crate::ca::CaScope::Global { "global" } else { "domains" },
        "domains": info.scope.domain_list(),
    }))
}
