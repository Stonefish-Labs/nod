use anyhow::Result;
use nod_client_core::{NodClientMessage, NodClientRuntime, RpcRequest, RpcResponse};
use serde_json::{json, Value};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    sync::mpsc,
};

#[tokio::main]
async fn main() -> Result<()> {
    let (stdout_tx, mut stdout_rx) = mpsc::channel::<Value>(128);
    let (message_tx, mut message_rx) = mpsc::channel::<NodClientMessage>(128);
    let message_stdout = stdout_tx.clone();
    tokio::spawn(async move {
        while let Some(message) = message_rx.recv().await {
            if message_stdout.send(json!(message)).await.is_err() {
                break;
            }
        }
    });

    tokio::spawn(async move {
        let mut stdout = tokio::io::stdout();
        while let Some(message) = stdout_rx.recv().await {
            let Ok(raw) = serde_json::to_vec(&message) else {
                continue;
            };
            if stdout.write_all(&raw).await.is_err()
                || stdout.write_all(b"\n").await.is_err()
                || stdout.flush().await.is_err()
            {
                break;
            }
        }
    });

    let mut runtime = NodClientRuntime::new(message_tx.clone()).await?;
    runtime.emit_ready().await;
    runtime.emit_state().await;

    let stdin = BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();
    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<RpcRequest>(&line) {
            Ok(request) => runtime.handle_rpc(request).await,
            Err(error) => RpcResponse::failure(Value::Null, error.to_string()),
        };
        let _ = stdout_tx.send(json!(response)).await;
    }
    runtime.disconnect_sync().await;
    Ok(())
}
