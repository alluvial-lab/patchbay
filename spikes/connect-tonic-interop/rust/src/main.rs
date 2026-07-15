// Throwaway spike server for the @connectrpc/connect-node <-> tonic interop
// validation (story-connect-node-tonic-interop-spike). NOT production code.
//
// Validates the conditions the seam will actually exercise:
//  - Unary RPC over gRPC/HTTP2
//  - Server-streaming RPC (Subscribe(cursor) -> stream SubscribeEvent)
//  - Richer gRPC error model: google.rpc.Status with a custom detail, carried
//    in the grpc-status-details-bin trailer (the most likely interop seam to
//    break per the spike brief).
//  - Request metadata propagation (operator-session / CSRF-evidence shape).
//  - TLS (when run with `tls` mode).
//
// Run modes (first arg):
//  - "plain" : HTTP/2 cleartext (h2c) on 127.0.0.1:PORT     [conditions 1-4]
//  - "tls"   : HTTP/2 over TLS on 127.0.0.1:PORT            [condition 5]
// PORT env (default 50051); for tls, TLS_CERT / TLS_KEY env must point to a
// cert/key pair.

tonic::include_proto!("patchbay.spike");

use std::{env, pin::Pin, time::Duration};

use prost::Message;
use prost_types::Any as ProtoAny;
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, Stream};
use tonic::{transport::Server, Request, Response, Status};

// The google.rpc.Status message, provided by tonic-types.
use tonic_types::pb::Status as RpcStatus;

#[derive(Default)]
pub struct SpikeService;

type SubscribeStream =
    Pin<Box<dyn Stream<Item = Result<SubscribeEvent, Status>> + Send + 'static>>;

#[tonic::async_trait]
impl spike_control_server::SpikeControl for SpikeService {
    async fn submit(
        &self,
        req: Request<SubmitRequest>,
    ) -> Result<Response<SubmitResult>, Status> {
        // Condition 4: metadata propagation. Echo back what arrived so the
        // client can assert the metadata it set actually reached the handler.
        let md = req.metadata();
        let op_session = md
            .get("x-patchbay-operator-session")
            .map(|v| v.to_str().unwrap_or("").to_string())
            .unwrap_or_default();
        let csrf = md
            .get("x-patchbay-csrf")
            .map(|v| v.to_str().unwrap_or("").to_string())
            .unwrap_or_default();

        let inner = req.into_inner();

        // Condition 3: error mapping. Return a richer gRPC error whose
        // grpc-status-details-bin trailer carries a serialized google.rpc.Status
        // containing our custom SubmissionFailureDetail as an Any detail.
        if inner.trigger_error {
            let detail = SubmissionFailureDetail {
                command_id: inner.command_id.clone(),
                failure_code: "FAILURE_CODE_VALIDATION_FAILED".to_string(),
                reason: format!("rejected by spike; op_session={op_session} csrf={csrf}"),
            };

            // Encode the custom detail and wrap it as a protobuf Any. The
            // type_url MUST carry the message's full name so a client can
            // dispatch on it.
            let mut buf = Vec::new();
            detail
                .encode(&mut buf)
                .map_err(|e| Status::internal(format!("encode detail: {e}")))?;
            let any = ProtoAny {
                // Manual type URL: the generated structs derive prost::Message
                // but not prost::Name, so we compose the conventional
                // type.googleapis.com/<package>.<Message> form by hand. A client
                // dispatches on this string.
                type_url: "type.googleapis.com/patchbay.spike.SubmissionFailureDetail".to_string(),
                value: buf,
            };

            // Build google.rpc.Status (code = 3 = INVALID_ARGUMENT) and
            // serialize it into the Status details blob tonic puts in the
            // grpc-status-details-bin trailer.
            let rpc = RpcStatus {
                code: tonic::Code::InvalidArgument as i32,
                message: "submission rejected".to_string(),
                details: vec![any],
            };
            let mut rpc_buf = Vec::new();
            rpc.encode(&mut rpc_buf)
                .map_err(|e| Status::internal(format!("encode rpc status: {e}")))?;

            return Err(Status::with_details(
                tonic::Code::InvalidArgument,
                "submission rejected".to_string(),
                rpc_buf.into(),
            ));
        }

        // Condition 1: unary round-trip. Echo metadata into the diagnostic so
        // the client can confirm the full path worked.
        Ok(Response::new(SubmitResult {
            command_id: inner.command_id,
            accepted: true,
            accepted_lsn: 42,
            diagnostic: format!("ok; op_session={op_session} csrf={csrf}"),
        }))
    }

    type SubscribeStream = SubscribeStream;

    async fn subscribe(
        &self,
        req: Request<SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        let inner = req.into_inner();
        let count = inner.event_count.max(1) as usize;
        let start_lsn = inner.cursor;

        let (tx, rx) = mpsc::channel(count);
        tokio::spawn(async move {
            for i in 0..count {
                let lsn = start_lsn + i as u64 + 1;
                let ev = SubscribeEvent {
                    lsn,
                    payload: format!("event-{i}"),
                };
                // Condition 2: server-streaming. Small delay exercises real
                // async streaming over HTTP/2 frames rather than a fused batch.
                tokio::time::sleep(Duration::from_millis(5)).await;
                if tx.send(Ok(ev)).await.is_err() {
                    break;
                }
            }
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mode = std::env::args().nth(1).unwrap_or_else(|| "plain".to_string());
    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(50051);
    let addr = format!("127.0.0.1:{port}").parse()?;

    let svc = spike_control_server::SpikeControlServer::new(SpikeService::default());

    match mode.as_str() {
        "plain" => {
            println!("spike-server: h2c on {addr}");
            Server::builder().add_service(svc).serve(addr).await?;
        }
        "tls" => {
            let cert = env::var("TLS_CERT").expect("TLS_CERT env required for tls mode");
            let key = env::var("TLS_KEY").expect("TLS_KEY env required for tls mode");
            let identity = tonic::transport::Identity::from_pem(read_file(&cert), read_file(&key));
            let tls_config = tonic::transport::ServerTlsConfig::new().identity(identity);
            println!("spike-server: TLS on {addr}");
            Server::builder()
                .tls_config(tls_config)?
                .add_service(svc)
                .serve(addr)
                .await?;
        }
        other => {
            return Err(format!("unknown mode: {other} (use plain|tls)").into());
        }
    }
    Ok(())
}

fn read_file(p: &str) -> Vec<u8> {
    std::fs::read(p).unwrap_or_else(|e| panic!("read {p}: {e}"))
}
