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
        let Response::DBFullTimeRange(result) = response else {
            return Err(
                "unexpected response type for DBFullTimeRange request".into()
            );
        };
        result
    }

    /// Center and zoom fitting all data in the current time range and
    /// filters, for the zoom-all-data button. None if there is no data or
    /// the response type is wrong.
    pub async fn get_data_view_params(&self) -> Option<(common::LngLat, f64)> {
        let response = self.send_request(Request::DataViewParams).await;
        let Response::DataViewParams(result) = response else {
            tracing::error!(
                "unexpected response type for DataViewParams request"
            );
            return None;
        };
        result
    }
}
