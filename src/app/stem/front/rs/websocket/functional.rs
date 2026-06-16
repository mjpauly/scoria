use futures::channel::oneshot;

use common::{
    ws_messages::{Request, Response},
    ToBack,
};

use super::WebsocketService;

impl WebsocketService {
    async fn send_request(&self, request: Request) -> Response {
        let (tx, rx) = oneshot::channel();
        let id = uuid::Uuid::new_v4();
        self.pending_requests.borrow_mut().insert(id, tx);
        self.send_msg(ToBack::Request(id, request));
        rx.await.unwrap()
    }

    pub async fn get_db_full_time_range(
        &self,
    ) -> Result<Option<common::TimeRange>, String> {
        let response = self.send_request(Request::DBFullTimeRange).await;
        #[allow(irrefutable_let_patterns)] // just a small enum right now
        let Response::DBFullTimeRange(result) = response
        else {
            // anyhow::bail!("unexpected response type for DBFullTimeRange request");
            return Err(
                "unexpected response type for DBFullTimeRange request".into()
            );
        };
        result
    }
}
