use core::{bootstrap, node_controller::create_node};
use std::path::PathBuf;

use kube::api::ListParams;

#[tokio::main]
async fn main() ->anyhow::Result<()> {
    println!("Hello, world!");
    
    match std::fs::metadata("C:\\Users\\wangyiwei\\Desktop\\coding\\redged\\.kube\\kubeconfig"){
        Ok(_)=>{
            std::env::set_var("KUBECONFIG", "C:\\Users\\wangyiwei\\Desktop\\coding\\redged\\.kube\\kubeconfig")
        },
        Err(_)=>{
            bootstrap::bootstrap(
                "bk6qog.3xt1gmto4nph658a",
                 "wywk8snode",
                  "https://10.101.12.130:6443", 
                  &PathBuf::from(".\\.kube\\ca.crt")).await;
        },
    }
    let kubeconfig = kube::config::Config::infer().await?;
    let client = kube::Client::try_from(kubeconfig)?;
    create_node(&client).await?;
    return Ok(());

    use k8s_openapi::api::core::v1::Pod;
    let all_pod_client: kube::Api<Pod> = kube::Api::all(client);
    let pods = all_pod_client.list(&ListParams::default()).await?;
    println!("=========获取了Pod列表{:?}", pods);
    Ok(())
}
