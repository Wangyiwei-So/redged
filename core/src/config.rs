use std::{net::IpAddr, path::PathBuf};

#[derive(Clone,Debug)]
pub struct Config{
    pub node_ip: IpAddr,
    pub hostname: String,
    pub node_name: String,
    pub data_dir: PathBuf,
}

#[derive(Debug, Default)]
struct  ConfigBuilder{
    pub node_ip: Option<IpAddr>,
    pub hostname: Option<String>,
    pub node_name: Option<String>,
    pub data_dir: Option<PathBuf>,
}