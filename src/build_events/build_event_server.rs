use tonic::{transport::Server, Request, Response, Status};

use crate::protos::*;

use futures::{Stream, StreamExt};
use tokio::prelude::*;

use google::devtools::build::v1::publish_build_event_server::{
    PublishBuildEvent, PublishBuildEventServer,
};
use google::devtools::build::v1::{
    PublishBuildToolEventStreamRequest, PublishBuildToolEventStreamResponse,
    PublishLifecycleEventRequest,
};
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;

pub mod bazel_event {
    use super::*;
    use ::prost::Message;

    #[derive(Clone, PartialEq, Debug)]
    pub struct BazelEvent {
        pub event: Evt,
    }
    impl BazelEvent {
        pub fn transform_from(
            inbound_evt: &mut PublishBuildToolEventStreamRequest,
        ) -> Option<BazelEvent> {
            let mut inner_data = inbound_evt
                .ordered_build_event
                .take()
                .as_mut()
                .and_then(|mut inner| inner.event.take());
            let _event_time = inner_data.as_mut().and_then(|e| e.event_time.take());
            let _event = inner_data.and_then(|mut e| e.event.take());

            let decoded_evt = match _event {
                Some(inner) => match inner {
                    google::devtools::build::v1::build_event::Event::BazelEvent(e) => {
                        Evt::BazelEvent(build_event_stream::BuildEvent::decode(&*e.value).unwrap())
                    }
                    other => Evt::UnknownEvent(format!("{:?}", other)),
                },
                None => Evt::UnknownEvent("Missing Event".to_string()),
            };

            Some(BazelEvent { event: decoded_evt })
        }
    }
    #[derive(Clone, PartialEq, Debug)]
    pub enum Evt {
        BazelEvent(build_event_stream::BuildEvent),
        UnknownEvent(String),
    }
}

pub enum BuildEventAction<T> {
    BuildEvent(T),
    LifecycleEvent(PublishLifecycleEventRequest),
    BuildCompleted,
}

pub struct BuildEventService<T>
where
    T: Send + Sync + 'static,
{
    pub write_channel: mpsc::Sender<BuildEventAction<T>>,
    pub transform_fn:
        Arc<dyn Fn(&mut PublishBuildToolEventStreamRequest) -> Option<T> + Send + Sync>,
}

fn transform_queue_error_to_status() -> Status {
    Status::resource_exhausted("Exhausted queue when trying to publish message")
}

pub fn build_bazel_build_events_service() -> (
    BuildEventService<bazel_event::BazelEvent>,
    mpsc::Receiver<BuildEventAction<bazel_event::BazelEvent>>,
) {
    let (tx, mut rx) = mpsc::channel(256);
    let server_instance = BuildEventService {
        write_channel: tx,
        transform_fn: Arc::new(bazel_event::BazelEvent::transform_from),
    };
    (server_instance, rx)
}

#[tonic::async_trait]
impl<T> PublishBuildEvent for BuildEventService<T>
where
    T: Send + Sync + Clone,
{
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
        let transform_fn = Arc::clone(&self.transform_fn);
        let output = async_stream::try_stream! {
            while let Some(inbound_evt) = stream.next().await {
                let mut inbound_evt = inbound_evt?;

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
                let transformed_data = (transform_fn)(&mut inbound_evt);

                if let Some(r) = transformed_data {
                    cloned_v.send(BuildEventAction::BuildEvent(r)).await.map_err(|_| transform_queue_error_to_status())?;
                }
            }
            // cloned_v.send(BuildEventAction::BuildCompleted).await.map_err(|e| transform_queue_error_to_status(e))?;

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
        // cloned_v
        // .send(BuildEventAction::LifecycleEvent(request.into_inner()))
        // .await
        // .map_err(|e| transform_queue_error_to_status(e))?;
        Ok(Response::new(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::prost::Message;

    use futures::future;
    use futures::future::FutureExt;
    use futures::stream;
    use futures::StreamExt;
    use pinky_swear::{Pinky, PinkySwear};
    use std::convert::TryFrom;
    use std::io::Read;
    use std::path::PathBuf;
    use std::time::Duration;
    use tempfile::tempdir;
    use tokio::net::UnixListener;
    use tokio::net::UnixStream;
    use tokio::sync::mpsc;
    use tokio::time;
    use tonic::transport::Server;
    use tonic::transport::{Endpoint, Uri};
    use tonic::Request;
    use tower::service_fn;

    fn load_proto(name: &str) -> Vec<PublishBuildToolEventStreamRequest> {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/tests/build_events");
        d.push(name);

        let mut file = std::fs::File::open(d).expect("Expected to be able to open input test data");

        let mut data_vec = vec![];
        let _ = file
            .read_to_end(&mut data_vec)
            .expect("Expected to read file");

        let mut buf: &[u8] = &data_vec;
        let mut res_buf = vec![];

        while buf.len() > 0 {
            res_buf.push(
                PublishBuildToolEventStreamRequest::decode_length_delimited(&mut buf).unwrap(),
            );
        }
        res_buf
    }

    struct ServerStateHandler {
        _temp_dir_for_uds: tempfile::TempDir,
        completion_pinky: Pinky<()>,
        pub read_channel: Option<mpsc::Receiver<BuildEventAction<bazel_event::BazelEvent>>>,
    }
    impl Drop for ServerStateHandler {
        fn drop(&mut self) {
            self.completion_pinky.swear(());
            // let the server shutdown gracefully before we cleanup the tempdir
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    use futures::TryStreamExt;
    use google::devtools::build::v1::publish_build_event_client;
    use google::devtools::build::v1::publish_build_event_server;

    async fn make_test_server() -> (
        ServerStateHandler,
        publish_build_event_client::PublishBuildEventClient<tonic::transport::channel::Channel>,
    ) {
        let uds_temp_dir = tempdir().unwrap();

        let path = uds_temp_dir.path().join("server_path");
        let path_copy = path.clone();
        println!("Path: {:?}", path);

        let (server_instance, mut rx) = build_bazel_build_events_service();

        let (promise, completion_pinky) = PinkySwear::<()>::new();
        let server_state = ServerStateHandler {
            _temp_dir_for_uds: uds_temp_dir,
            completion_pinky: completion_pinky,
            read_channel: Some(rx),
        };
        // let shutdown_promise =
        tokio::spawn(async {
            let mut uds = UnixListener::bind(path).expect("Should be able to setup unix listener");

            eprintln!("Starting server..");
            Server::builder()
                .add_service(publish_build_event_server::PublishBuildEventServer::new(
                    server_instance,
                ))
                .serve_with_incoming_shutdown(
                    uds.incoming().map_ok(crate::tokioext::unix::UnixStream),
                    promise,
                )
                .inspect(|x| println!("resolving future: {:?}", &x))
                .await
                .expect("Failed to start server")
        });

        time::delay_for(Duration::from_millis(5)).await;

        let endpoint: Endpoint =
            Endpoint::try_from("lttp://[::]:50051").expect("Can calculate endpoint");

        let channel: tonic::transport::channel::Channel = endpoint
            .connect_with_connector(service_fn(move |_: Uri| {
                let path_copy = path_copy.clone();
                // Connect to a Uds socket
                UnixStream::connect(path_copy)
            }))
            .await
            .expect("Connect to server");

        let client: publish_build_event_client::PublishBuildEventClient<
            tonic::transport::channel::Channel,
        > = publish_build_event_client::PublishBuildEventClient::new(channel);

        (server_state, client)
    }

    #[tokio::test]
    async fn test_no_op_build_stream() {
        let event_stream = load_proto("no_op_build.proto");
        let (mut state, mut client) = make_test_server().await;

        let stream = stream::iter(event_stream.clone());
        let ret_v = client
            .publish_build_tool_event_stream(Request::new(stream))
            .await
            .expect("service call should succeed")
            .into_inner();

        // need to exhaust the stream to ensure we complete the operation
        ret_v.for_each(|ret_v| future::ready(())).await;

        let mut data_stream = vec![];
        let mut channel = state.read_channel.take().unwrap();

        tokio::spawn(async move {
            std::thread::sleep(Duration::from_millis(20));
            drop(state);
        });

        while let Some(action) = channel.recv().await {
            match action {
                BuildEventAction::BuildCompleted => (),
                BuildEventAction::LifecycleEvent(_) => (),
                BuildEventAction::BuildEvent(msg) => {
                    data_stream.push(msg);
                }
            }
        }

        assert_eq!(event_stream.len(), data_stream.len());
        for e in data_stream {
            println!("{:?}", e);
        }
        // assert_eq!(3, 5);
        // assert_eq!(event_stream, data_stream);
    }
}
