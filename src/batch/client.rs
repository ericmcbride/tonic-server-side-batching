use hello_world::greeter_client::GreeterClient;
use hello_world::HelloRequest;
use futures::future;
use std::time::Instant;


pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = GreeterClient::connect("http://[::1]:50051").await?;
    let mut tasks = vec![];

    for n in 1..10000 {
        let mut client = client.clone();
        let request = tonic::Request::new(HelloRequest {
            name: format!("Tonic-{}", n),
        });

        tasks.push(tokio::task::spawn(async move {
            let _response = client.say_hello(request).await.unwrap();
        }));
    }

    let now = Instant::now();
    println!("Running tasks");
    let _ = future::join_all(tasks).await;
    println!("{}ms elapsed", now.elapsed().as_millis());
    Ok(())
}
