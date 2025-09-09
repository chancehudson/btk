use crate::app::ActionRequest;
use crate::app::AppEvent;
use crate::data::LocalState;
use crate::network::NetworkManager;

/// A state object that is accessible in all applets.
pub struct AppState {
    pub local_data: LocalState,
    pub pending_events: (flume::Sender<AppEvent>, flume::Receiver<AppEvent>),
    pub pending_requests: (flume::Sender<ActionRequest>, flume::Receiver<ActionRequest>),
}

impl AppState {
    pub fn drain_pending_app_events(&self) -> Vec<AppEvent> {
        self.pending_events.1.drain().collect()
    }

    pub fn switch_cloud(&self, id: [u8; 32]) {
        self.pending_requests
            .0
            .send(ActionRequest::SwitchCloud(id))
            .expect("failed to send app request");
    }

    pub fn reload_clouds(&self) {
        self.pending_requests
            .0
            .send(ActionRequest::LoadClouds)
            .expect("failed to send app request");
    }

    pub fn drain_pending_app_requests(&self) -> Vec<ActionRequest> {
        self.pending_requests.1.drain().collect()
    }
}
