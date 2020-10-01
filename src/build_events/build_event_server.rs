use tonic::{transport::Server, Request, Response, Status};

use crate::protos::*;
use ::prost::Message;
use futures::{Stream, StreamExt};
use tokio::prelude::*;

use std::pin::Pin;

use google::devtools::build::v1::publish_build_event_server::{
    PublishBuildEvent, PublishBuildEventServer,
};
use google::devtools::build::v1::{
    PublishBuildToolEventStreamRequest, PublishBuildToolEventStreamResponse,
    PublishLifecycleEventRequest,
};
use tokio::sync::mpsc;

pub enum BuildEventAction {
    BuildEvent(PublishBuildToolEventStreamRequest),
    LifecycleEvent(PublishLifecycleEventRequest),
    BuildCompleted,
}

pub struct BuildEventService {
    pub write_channel: mpsc::Sender<BuildEventAction>,
}

fn transform_queue_error_to_status(
    _: tokio::sync::mpsc::error::SendError<BuildEventAction>,
) -> Status {
    Status::resource_exhausted("Exhausted queue when trying to publish message")
}

#[tonic::async_trait]
impl PublishBuildEvent for BuildEventService {
    type PublishBuildToolEventStreamStream = Pin<
        Box<
            dyn Stream<Item = Result<PublishBuildToolEventStreamResponse, Status>>
                + Send
                + Sync
                + 'static,
        >,
    >;

    async fn publish_build_tool_event_stream(
        &self,
        request: Request<tonic::Streaming<PublishBuildToolEventStreamRequest>>,
    ) -> Result<Response<Self::PublishBuildToolEventStreamStream>, Status> {
        let mut stream = request.into_inner();
        let mut cloned_v = self.write_channel.clone();

        let output = async_stream::try_stream! {
            while let Some(inbound_evt) = stream.next().await {
                let inbound_evt = inbound_evt?;

                match inbound_evt.ordered_build_event.as_ref() {
                    Some(build_event) => {
                    let sequence_number = build_event.sequence_number;
                let stream_id = build_event.stream_id.clone();

                yield PublishBuildToolEventStreamResponse {
                    stream_id: stream_id,
                    sequence_number: sequence_number
                };
            }
                    None => ()
                };
                cloned_v.send(BuildEventAction::BuildEvent(inbound_evt)).await.map_err(|e| transform_queue_error_to_status(e))?;
            }
            cloned_v.send(BuildEventAction::BuildCompleted).await.map_err(|e| transform_queue_error_to_status(e))?;

        };

        Ok(Response::new(
            Box::pin(output) as Self::PublishBuildToolEventStreamStream
        ))
    }

    async fn publish_lifecycle_event(
        &self,
        request: tonic::Request<PublishLifecycleEventRequest>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        let mut cloned_v = self.write_channel.clone();
        cloned_v
            .send(BuildEventAction::LifecycleEvent(request.into_inner()))
            .await
            .map_err(|e| transform_queue_error_to_status(e))?;
        Ok(Response::new(()))
    }
}
