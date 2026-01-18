use std::sync::Arc;

use crate::network::node::NodeManage;
use tokio::time::{Duration, interval};

impl NodeManage{
    pub async fn start_miner(self:Arc<Self>){
        let mut timer = interval(Duration::from_secs(10));
    }
}