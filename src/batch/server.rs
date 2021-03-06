use tonic::{transport::Server, Request, Response, Status};
use tonic::Code;

use hello_world::greeter_server::{Greeter, GreeterServer};
use hello_world::{HelloReply, HelloRequest};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, info, Level};
use tracing_subscriber;
use uuid::Uuid;
use ringbuf::RingBuffer;
use std::collections::VecDeque;
use std::time::Duration;


pub mod hello_world {
    tonic::include_proto!("helloworld");
}

/// A thin wrapper, so we can send over the request, and the Sender channel, so after the
/// batching, we are prepared to send back the single response
#[derive(Debug)]
pub struct RequestWrapper {
    single_shot: oneshot::Sender<String>,
    req: Request<HelloRequest>,
}

/// Hello World endpoint
#[derive(Debug)]
pub struct MyGreeter {
    /// MPSC Sender to send requests to the batch scheduler
    inbound_sender: mpsc::Sender<RequestWrapper>,
}

#[tonic::async_trait]
impl Greeter for MyGreeter {
    /// Basic hello function.  Here we create a one shot channel pair (to communicate back after
    /// the batch request) and we create a batch wrapper object and send it over to the
    /// batch_scheduler for processing.  Returns a number
    #[tracing::instrument]
    async fn say_hello(
        &self,
        mut request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        let (tx, rx) = oneshot::channel::<String>();
        let header_resp = request.metadata().get("extra-id".to_string()).unwrap().as_bytes();
        let header_str = std::str::from_utf8(header_resp).unwrap().to_string();

        let mut request_wrapper = RequestWrapper {
            single_shot: tx,
            req: request,
        };

        let _ = self.inbound_sender.clone().send(request_wrapper).await;
        match rx.await {
            Ok(result) => {
                let reply = hello_world::HelloReply {
                    message: format!("Number is {}-{}", result, header_str),
                };
                Ok(Response::new(reply))
            }
            Err(e) => {
                let status = Status::new(Code::Aborted, format!("Single Shot closed {}", e));
                Err(status)
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .init();

    let addr = "[::1]:50051".parse()?;
    let (inbound_tx, inbound_rx) = mpsc::channel::<RequestWrapper>(100000000);

    let greeter = MyGreeter {
        inbound_sender: inbound_tx,
    };

    let svc = GreeterServer::with_interceptor(greeter, header_interceptor);

    tokio::spawn(async move { batch_scheduler(inbound_rx).await });
    Server::builder()
        .add_service(svc)
        .serve(addr)
        .await?;

    Ok(())
}

/// Background Task to intercept GRPC requests for batching
#[tracing::instrument]
async fn batch_scheduler(mut rx: mpsc::Receiver<RequestWrapper>) {
    info!("Starting batcher");

    let buff = RingBuffer::<RequestWrapper>::new(10000000000);
    let (mut prod, mut cons) = buff.split();

    // TODO: Dont use delay use some kind of interval ticker
    let mut delay = tokio::time::delay_for(Duration::from_millis(500));
    loop {
        tokio::select! {
            _ = &mut delay => {
                let cons_len = cons.len();
                if cons_len == 0 {
                    continue;
                }
                debug!("Batch Buffer timeout hit with {} requests", cons_len);
                let _ = batch_fan_out(&mut cons, cons_len);
            }

            Some(new_req) = rx.recv() => {
                prod.push(new_req).unwrap();
                if cons.len() == 1000 {
                    let _ = batch_fan_out(&mut cons, 1000);
                }
            }
        }
        delay = tokio::time::delay_for(Duration::from_millis(500));
    }
}

/// Function that will send the requests back to their respected client request
fn batch_fan_out(cons: &mut ringbuf::Consumer<RequestWrapper>, size: usize) -> Result<(), Box<dyn std::error::Error>>{
    let mut batch_vec = VecDeque::new();
    let collector = |val| {
        batch_vec.push_back(val);
        true
    };

    let _ = cons.pop_each(collector, Some(size));
    let results = batch_request(&batch_vec)?;
    for result in results.into_iter() {
        let chan = batch_vec.pop_front().unwrap();
        let _ = chan.single_shot.send(result)?;
    }
    Ok(())
}

/// gRPC Interceptor used to Implement Custom id for fan out method in the batch scheduler.  This
/// can be deleted, only making sure order is preserved in this implementation
fn header_interceptor(mut req: Request<()>) -> Result<Request<()>, Status> {
    let id = Uuid::new_v4().to_hyphenated().to_string();
    req.metadata_mut().insert("extra-id", id.parse().unwrap());
    Ok(req)
}

/// Simulates posting a batch request, then returning a batch request
fn batch_request(batch: &VecDeque<RequestWrapper>) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut req_vec = vec![];
    for r in batch.iter() {
        let val = r.req.metadata().get("extra-id".to_string()).unwrap().as_bytes();
        req_vec.push(std::str::from_utf8(val)?.to_string());
    }
    Ok(req_vec)
}
