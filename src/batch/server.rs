use tonic::{transport::Server, Request, Response, Status};

use hello_world::greeter_server::{Greeter, GreeterServer};
use hello_world::{HelloReply, HelloRequest};
use tokio::sync::{mpsc, oneshot};
use tokio::time::{self, Duration};
use tracing::{debug, error, event, info, Level};
use tracing_subscriber;
use uuid::Uuid;
use ringbuf::RingBuffer;

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

/// A thin wrapper, so we can send over the request, and the Sender channel, so after the
/// batching, we are prepared to send back the single response
#[derive(Debug)]
pub struct RequestWrapper {
    single_shot: oneshot::Sender<i32>,
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
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        info!("request made {:?}", request);
        let (tx, rx) = oneshot::channel::<i32>();
        let request_wrapper = RequestWrapper {
            single_shot: tx,
            req: request,
        };

        let _ = self.inbound_sender.clone().send(request_wrapper).await;
        //let resp = rx.await;
        //info!("Resp is {:?}", resp);
        let reply = hello_world::HelloReply {
            message: format!("Number is 1"),
        };
        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .init();

    let addr = "[::1]:50051".parse().unwrap();
    let (inbound_tx, inbound_rx) = mpsc::channel::<RequestWrapper>(10000);

    let greeter = MyGreeter {
        inbound_sender: inbound_tx,
    };
    
    let svc = GreeterServer::with_interceptor(greeter, append_id);

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
    
    let buff = RingBuffer::<RequestWrapper>::new(1000000);
    let (mut prod, mut cons) = buff.split();

    // Replace with a ticker of some sort
    let mut delay = time::delay_for(Duration::from_millis(5000));
    loop {
        tokio::select! {
            Some(new_req) = rx.recv() => {
                prod.push(new_req).unwrap();
                if cons.len() == 10 {
                    let mut batch_vec = vec![];
                    let collector = |val| {
                        batch_vec.push(val);
                        true
                    };
                    let _ = cons.pop_each(collector, Some(10));
                   
                    // TODO: Build out batch request, and singleshot the request back to the
                    // handler.
                    println!("Popped off cons is {:?}", batch_vec);
                    println!("length of consumer is 1 lol");
                
                }
                // Here you would add to check the length of a batch, then add to it, or process it
                continue
            }
        }
    }
}

/// gRPC Interceptor used to Implement Custom id for fan out method in the batch scheduler
#[tracing::instrument]
fn append_id(mut req: Request<()>) -> Result<Request<()>, Status> {
    let id = Uuid::new_v4().to_hyphenated().to_string();
    req.metadata_mut().insert("extra-id", id.parse().unwrap());
    Ok(req)
}

#[tracing::instrument]
fn batch_request() -> Result<(), Box<dyn std::error::Error>> {
    // simulate latency and create batches with random numbers
    Ok(())
}
