use tonic::{transport::Server, Request, Response, Status};

use hello_world::greeter_server::{Greeter, GreeterServer};
use hello_world::{HelloReply, HelloRequest};
use tokio::sync::{mpsc, oneshot};
use tokio::time::{self, Duration};

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

/// A thin wrapper, so we can send over the request, and the Sender channel, so after the
/// batching, we are prepared to send back the single response
#[derive(Debug)]
pub struct BatchWrapper {
    single_shot: oneshot::Sender<i32>,
    req: Request<HelloRequest>,
}

/// Hello World endpoint 
pub struct MyGreeter {
    /// MPSC Sender to send requests to the batch scheduler 
    inbound_sender: mpsc::Sender<BatchWrapper>,
}

#[tonic::async_trait]
impl Greeter for MyGreeter {
    /// Basic hello function.  Here we create a one shot channel pair (to communicate back after
    /// the batch request) and we create a batch wrapper object and send it over to the
    /// batch_scheduler for processing.  Returns a number
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        println!("request made");
        let (tx, rx) = oneshot::channel::<i32>();
        let batch_wrapper = BatchWrapper {
            single_shot: tx,
            req: request,
        };

        let _ = self.inbound_sender.clone().send(batch_wrapper).await;
        let resp = rx.await;

        println!("Resp is {:?}", resp);
        let reply = hello_world::HelloReply {
            message: format!("Number is {}", resp.unwrap()),
        };
        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse().unwrap();
    let (inbound_tx, inbound_rx) = mpsc::channel::<BatchWrapper>(100);

    let greeter = MyGreeter {
        inbound_sender: inbound_tx,
    };

    tokio::spawn(async move { batch_scheduler(inbound_rx).await });
    Server::builder()
        .add_service(GreeterServer::new(greeter))
        .serve(addr)
        .await?;
    Ok(())
}

/// Background Task to intercept GRPC requests for batching
async fn batch_scheduler(mut rx: mpsc::Receiver<BatchWrapper>) {
    println!("Starting batcher");
    let mut delay = time::delay_for(Duration::from_millis(5000));
    loop {
        tokio::select! {
            _ = &mut delay => {}
            Some(new_req) = rx.recv() => {
                println!("Received new request");
                // Here you would add to check the length of a batch, then add to it, or process it
                let _ = new_req.single_shot.send(1);
                continue
            }
        }
    }
}
