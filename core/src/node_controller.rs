use std::collections::BTreeMap;
use chrono::Utc;
use anyhow::{Result, Ok};
use k8s_openapi::{api::{core::v1::{Node, NodeSpec, NodeStatus, NodeCondition, NodeSystemInfo}, coordination::v1::{Lease, LeaseSpec}}, apimachinery::pkg::apis::meta::v1::{Time, OwnerReference, MicroTime}};
use kube::{Api, api::{PostParams, PatchParams}};
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
pub async fn create_node(client: &kube::Client)->Result<()>{
    let node_client: Api<Node> = Api::all(client.clone());

    let node_name = "wywk8snode";
    
    // 如果node存在就跳过
    if node_client.get(node_name).await.is_ok(){
        println!("node wywk8snode 已经存在，跳过创建node");
        return Ok(());
    }

    let mut node = Node::default();
    node.metadata.name = Some(node_name.into());
    node.metadata.annotations = Some(vec![
        (String::from("node.alpha.kubernetes.io/ttl"), String::from("0")),
        (String::from("volumes.kubernetes.io/controller-managed-attach-detach"), String::from("true")),
    ].into_iter().collect());

    let arch = std::env::consts::ARCH;
    let os = std::env::consts::OS;
    let hostname =hostname::get()?.to_string_lossy().to_string();
    node.metadata.labels = Some(vec![
        ("beta.kubernetes.io/os".into(), os.into()),
        ("kubernetes.io/os".into(), os.into()),
        ("beta.kubernetes.io/arch".into(), arch.into()),
        ("kubernetes.io/arch".into(), arch.into()),
        ("kubernetes.io/hostname".into(), hostname),
        ("type".into(), "wywedged".into()),
    ].into_iter().collect());
    let mut node_spec = NodeSpec::default();
    node.spec = Some(node_spec);
    let mut node_status = NodeStatus::default();
    node_status.capacity = Some(vec![
        ("cpu".into(),Quantity("4".into())), 
        ("ephemeral-storage".into(),Quantity("61255492Ki".into())),
        ("hugepages-1Gi".into(),Quantity("0".into())),
        ("hugepages-2Mi".into(),Quantity("0".into())),
        ("memory".into(),Quantity("4032800Ki".into())),
        ("pods".into(),Quantity("100".into())),
    ].into_iter().collect());
    node_status.allocatable = Some(vec![
        ("cpu".into(),Quantity("4".into())), 
        ("ephemeral-storage".into(),Quantity("61255492Ki".into())),
        ("hugepages-1Gi".into(),Quantity("0".into())),
        ("hugepages-2Mi".into(),Quantity("0".into())),
        ("memory".into(),Quantity("4032800Ki".into())),
        ("pods".into(),Quantity("100".into())),
    ].into_iter().collect());
    node_status.conditions = Some(vec![
        NodeCondition {
            type_: "Ready".into(),
            status: "True".into(),
            last_heartbeat_time: Some(Time(Utc::now())),
            last_transition_time: Some(Time(Utc::now())),
            reason: Some("EdgedReady".into()),
            message: Some("wyw redged is ready".into()),
        },
        NodeCondition {
            type_: "DiskPressure".into(),
            status: "False".into(),
            last_heartbeat_time: Some(Time(Utc::now())),
            last_transition_time: Some(Time(Utc::now())),
            reason: Some("KubeletHasSufficientDisk".into()),
            message: Some("wyw redged has sufficient disk space available".into()),
        },
        NodeCondition {
            type_: "MemoryPressure".into(),
            status: "False".into(),
            last_heartbeat_time: Some(Time(Utc::now())),
            last_transition_time: Some(Time(Utc::now())),
            reason: Some("KubeletHasSufficientMemory".into()),
            message: Some("wyw redged has sufficient memory available".into()),
        },
        NodeCondition {
            type_: "PIDPressure".into(),
            status: "False".into(),
            last_heartbeat_time: Some(Time(Utc::now())),
            last_transition_time: Some(Time(Utc::now())),
            reason: Some("KubeletHasSufficientPID".into()),
            message: Some("wyw redged has sufficient pid available".into()),
        },
    ]);

    node_status.node_info = Some(NodeSystemInfo{
        kubelet_version: "v0.1".into(),
        ..Default::default()
    });
    node.status = Some(node_status);
    node_client.create(&PostParams::default(), &node).await?;

    println!("node创建成功");
    Ok(())
}

pub async fn update(client: &kube::Client, node_name: &str){
    println!("更新node状态");
    
}

async fn update_lease(node_uid: &str, node_name: &str, client: &kube::Client)->Result<()>{
    let lease_client: Api<Lease> = Api::namespaced(client.clone(), "kube-node-lease");
    let mut lease = Lease::default();
    lease.metadata.name = Some(node_name.into());
    lease.metadata.owner_references = Some(vec![OwnerReference{
        api_version: "v1".into(),
        kind: "Node".into(),
        name: node_name.into(),
        uid: node_uid.into(),
        ..Default::default()
    }]);
    let now = Utc::now();
    lease.spec = Some(LeaseSpec{
        holder_identity: Some(node_name.into()),
        acquire_time: Some(MicroTime(now)),
        renew_time: Some(MicroTime(now)),
        lease_duration_seconds: Some(300),
        ..Default::default()
    });
    lease_client.patch(node_name,
         &PatchParams::default(), 
         &kube::api::Patch::Strategic(lease)).await?;
    Ok(())
}