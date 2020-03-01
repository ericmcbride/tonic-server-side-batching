use hello_world::greeter_client::GreeterClient;
use hello_world::HelloRequest;

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = GreeterClient::connect("http://[::1]:50051").await?;
    let mut tasks = vec![];

    for n in 1..100 {
        let mut client = client.clone();
        let request = tonic::Request::new(HelloRequest {
            name: format!("Tonic-{}", n),
        });

        tasks.push(tokio::task::spawn(async move {
            let response = client.say_hello(request).await.unwrap();
            println!("RESPONSE={:?}", response);
        }));
    }

    let _ = tokio::task::spawn(async {
        for task in tasks {
            task.await.unwrap();
        }
    })
    .await;

    Ok(())
}
