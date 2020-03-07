#![feature(test)]
extern crate test;
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


#[bench]
fn f_1_parallel_requests(b: &mut test::bench::Bencher) {
    bench_requests(b, 1);
}

#[bench]
fn f_50_parallel_requests(b: &mut test::bench::Bencher) {
    bench_requests(b, 50);
}

#[bench]
fn f_10_parallel_requests(b: &mut test::bench::Bencher) {
    bench_requests(b, 10);
}

#[tokio::main]
async fn bench_requests(b: &mut test::bench::Bencher, concurrency: u32) {
    let client = GreeterClient::connect("https://[::1]:50051").await.unwrap();

    b.iter(move || {
        let mut parallel = Vec::new();
        for _i in 0..concurrency {
            let mut client = client.clone();
            let request = tonic::Request::new(HelloRequest {
                    name: format!("Tonic"),
            });
            parallel.push(tokio::task::spawn(async move {
                client
                    .say_hello(request)
                    .await
            }));
        }

        async { let _ = future::join_all(parallel).await; }
    });
}
