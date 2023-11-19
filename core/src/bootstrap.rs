use std::{path::{PathBuf, Path}, str::FromStr, env, fs};
use anyhow::{Result, Ok};
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::{api::certificates::v1::{CertificateSigningRequest, CertificateSigningRequestSpec}, ByteString};
use kube::{Api, core::ObjectMeta, api::{PostParams, ListParams}, runtime::{watcher::{Event,self}, WatchStreamExt}, config::AuthInfo};
use secrecy::SecretString;
use rcgen::{Certificate, CertificateParams, DistinguishedName, DnType, KeyPair, PKCS_ECDSA_P256_SHA256};
use base64::{Engine as _, engine::{self, general_purpose}, alphabet};
fn gen_auth_cert(node_name:&str)->Result<Certificate>{
    let mut params = CertificateParams::default();
    params.not_before = time::OffsetDateTime::now_utc();
    params.not_before = time::OffsetDateTime::now_utc()+time::Duration::weeks(32);
    let mut distinguished_name = DistinguishedName::new();
    distinguished_name.push(DnType::OrganizationName, "system:nodes");
    distinguished_name.push(DnType::CommonName, &format!("system:node:{}", node_name));
    params.distinguished_name = distinguished_name;
    params.key_pair.replace(KeyPair::generate(&PKCS_ECDSA_P256_SHA256)?);
    params.alg = &PKCS_ECDSA_P256_SHA256;
    Ok(Certificate::from_params(params)?)
}
use std::io::Write;
fn save_private_key_to_file(private_key: &[u8], file_path: &str) -> Result<()> {
    let mut file = std::fs::File::create(file_path)?;
    file.write_all(private_key)?;
    Ok(())
}

pub async fn bootstrap(token:&str, node_name:&str, master_url: &str, cert_path: &PathBuf){
    if kubeconfig_exists(){
        println!("已经有config了");
    }else{
        let cert_bundle = gen_auth_cert(node_name).unwrap(); //生成私钥
        println!("生成了私钥 {}",cert_bundle.serialize_private_key_pem());
        save_private_key_to_file(&cert_bundle.serialize_private_key_pem().as_bytes(),"C:\\Users\\wangyiwei\\Desktop\\coding\\redged\\.kube\\client.key").unwrap();
        let bootstrap_client = new_for_bootstrap(token, master_url).await.unwrap();
        let csrs: Api<CertificateSigningRequest> = Api::all(bootstrap_client);
        println!("===============");
        let cccc = csrs.list(&ListParams::default()).await.unwrap();
        println!("获取了目前的csrs{:?}",cccc);

        let mut csr_obj = CertificateSigningRequest{
            metadata: ObjectMeta::default(),
            spec: CertificateSigningRequestSpec::default(),
            status: None
        };
        csr_obj.metadata.name = Some(String::from(node_name));
        let cert_bundle_bytes= cert_bundle.serialize_request_pem().unwrap().as_bytes().to_vec(); //生成CSR
        csr_obj.spec.request = ByteString(cert_bundle_bytes);
        csr_obj.spec.usages = Some(vec![String::from("client auth")]);
        csr_obj.spec.expiration_seconds = Some(3600);
        csr_obj.spec.signer_name = String::from("kubernetes.io/kube-apiserver-client");
        println!("准备创建csr对象{:?}",csr_obj);
        match csrs.create(&PostParams::default(), &csr_obj).await{
            Err(e)=>{
                println!("创建csr对象错误{:?}",e);
            },
            _ => {},
        }
        println!("CSR creation successful, waiting for certificate approval");
        let mut csr_watcher = watcher::watcher(csrs,watcher::Config::default()
        .fields(&format!("metadata.name={}", node_name))).boxed();
        
        while let Some(event) = csr_watcher.try_next().await.unwrap(){
            let status = match event {
                Event::Applied(m)=> m.status.unwrap(),
                Event::Restarted(mut certs)=>{
                    if certs.len() > 1 {
                        println!("错误了");
                        return;
                    }
                    certs.remove(0).status.unwrap()
                }
                Event::Deleted(_)=>{
                    println!("csr被删除了");
                    return;               
                }
            };
            if let Some(cert) = status.certificate{
                if let Some(v) = status.conditions{
                    if v.into_iter().any(|c|{
                        c.type_.as_str() == "Approved"
                    }){
                        println!("csr被批准了! {}",String::from_utf8(cert.0.clone()).unwrap());
                        save_private_key_to_file(&cert.0,"C:\\Users\\wangyiwei\\Desktop\\coding\\redged\\.kube\\client.crt").unwrap();
                        save_kubeconfig(master_url).unwrap();
                        break;
                    }
                }
            }
        }
    }
}

async fn new_for_bootstrap(token: &str, master_url: &str)->Result<kube::Client>{
    let mut bootstrap_kubeconfig = kube::config::Config::new(http::Uri::from_str(master_url)?);
    bootstrap_kubeconfig.auth_info.token = Some(SecretString::new(String::from(token)));
    let data = fs::read(Path::new("C:\\Users\\wangyiwei\\Desktop\\coding\\redged\\.kube\\ca.crt"))?;

    // env::set_var("KUBECONFIG", "C:\\Users\\wangyiwei\\Desktop\\coding\\redged\\.kube\\bootstrap.yaml");
    // let mut bootstrap_kubeconfig = kube::Config::infer().await?;
    
    let data = pem::parse_many(data)?.into_iter().map(|p|{
            p.contents().to_vec()
    }).collect();
    bootstrap_kubeconfig.root_cert = Some(data);
    // let data = pem::parse(data)?;
    // println!("pem parse后的是{:?}",data.to_string());
    // bootstrap_kubeconfig.root_cert = Some(vec![data.contents().to_vec()]);
    let client = kube::Client::try_from(bootstrap_kubeconfig)?;
    Ok(client)
}

fn save_kubeconfig(master_url: &str)->Result<()>{
    let context_name = "default-context";
    let cluster_name = "default-cluster";
    let auth_name = "default-auth";
    let mut config = kube::config::Kubeconfig::default();
    config.clusters = vec!{kube::config::NamedCluster{
        name: cluster_name.into(),
        cluster: Some(kube::config::Cluster{
            server: Some(master_url.into()),
            certificate_authority: Some("ca.crt".into()),
            ..Default::default()
        })
    }};
    config.kind = Some("Config".into());
    config.api_version = Some("v1".into());
    config.contexts = vec![kube::config::NamedContext{
        name: context_name.into(),
        context: Some(kube::config::Context{
            cluster: cluster_name.into(),
            user: auth_name.into(),
            ..Default::default()
        })
    }];

    config.auth_infos = vec![
        kube::config::NamedAuthInfo{
            name: auth_name.into(),
            auth_info: Some(kube::config::AuthInfo{
                client_certificate: Some("client.crt".into()),
                client_key: Some("client.key".into()),
                ..Default::default()
            }),
        }
    ];
    config.current_context = Some(context_name.into());
    let config_yaml = serde_yaml::to_string(&config).unwrap();
    save_private_key_to_file(&config_yaml.into_bytes(), "C:\\Users\\wangyiwei\\Desktop\\coding\\redged\\.kube\\kubeconfig")?;
    // println!("我生成了kubeconfig {:?}",config_yaml);
    Ok(())
}

// fn certs(data: &[u8]) -> Result<Vec<Vec<u8>>, pem::PemError> {
//     Ok(pem::parse_many(data)?
//         .into_iter()
//         .filter_map(|p| {
//             if p.tag() == "CERTIFICATE" {
//                 Some(p.into_contents())
//             } else {
//                 None
//             }
//         })
//         .collect::<Vec<_>>())
// }

fn kubeconfig_exists()->bool{
    false
}