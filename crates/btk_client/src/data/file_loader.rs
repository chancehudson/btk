use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

use anondb::Bytes;
use egui::load::BytesLoader;
use egui::load::BytesPoll;
use egui::load::LoadError;

use super::Cloud;

#[derive(Default)]
pub struct CloudFileLoader {
    active_cloud: RwLock<Option<Arc<Cloud>>>,
    data: RwLock<HashMap<String, Vec<u8>>>,
}

impl CloudFileLoader {
    pub fn set_active_cloud(&self, cloud_maybe: Option<Arc<Cloud>>) {
        *self.active_cloud.write().unwrap() = cloud_maybe;
        self.forget_all();
    }
}

impl BytesLoader for CloudFileLoader {
    fn id(&self) -> &str {
        "org.btkcloud.cloudfileloader"
    }

    fn load(&self, _ctx: &egui::Context, uri: &str) -> egui::load::BytesLoadResult {
        let name = uri.trim_start_matches("file://").to_string();
        if let Some(cloud) = self.active_cloud.read().unwrap().clone()
            && let Some(data) = cloud.db.get::<_, Bytes>("files", &name).ok().flatten()
        {
            self.data.write().unwrap().insert(name, data.to_vec());
            Ok(BytesPoll::Ready {
                size: None,
                bytes: data.to_vec().into(),
                mime: None,
            })
        } else {
            Err(LoadError::Loading("not found".into()))
        }
    }

    fn forget(&self, uri: &str) {
        self.data.write().unwrap().remove(uri);
    }

    fn end_pass(&self, _pass_index: u64) {}

    fn byte_size(&self) -> usize {
        0
    }

    fn forget_all(&self) {
        *self.data.write().unwrap() = HashMap::default();
    }

    fn has_pending(&self) -> bool {
        false
    }
}
