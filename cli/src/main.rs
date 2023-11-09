use core::bootstrap;
use std::path::PathBuf;

#[tokio::main]
async fn main() ->anyhow::Result<()> {
    println!("Hello, world!");
    bootstrap::bootstrap("6uc16e.0cdcrw5vb8qx4cvx", "wywk8snode", "https://10.101.12.130:6443", &PathBuf::from("./.kube/ca.crt")).await;
    Ok(())
}
