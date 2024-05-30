use core::time;

// use reqwest::{ClientBuilder, StatusCode};
use std::env;
use structopt::StructOpt;
// use bytes::Bytes;
use http_body_util::{BodyExt, Empty};
use hyper::{Request, StatusCode};
use tokio::io::{self, AsyncWriteExt as _};
use tokio::net::TcpStream;

use hyper::body::Bytes;
use hyper_util::rt::TokioIo;

#[derive(Debug, StructOpt, Clone)]
#[structopt(name = "tcp-listen", about = "An example of StructOpt usage.")]
struct Opt {
    /// Activate debug mode
    // short and long flags (-d, --debug) will be deduced from the field's name
    #[structopt(short = "q", long = "qps", default_value = "100")]
    tps: u64,

    // request sleep (ms)
    #[structopt(short = "t", long = "thread_num", default_value = "10")]
    thread_num: u64,

    // request level tcp or http (ms)
    #[structopt(short = "st", long = "syntm", default_value = "6000")]
    syn_timeout: u64,

    // request level tcp or http (ms)
    #[structopt(short = "rt", long = "reqtm", default_value = "6000")]
    request_timeout: u64,

    // request level tcp or http (ms)
    #[structopt(short = "u", long = "uri", default_value = "http://localhost:80/")]
    request_uri: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> { 
    let now = tokio::time::Instant::now();
    let url = "http://localhost:80/".parse::<hyper::Uri>().unwrap();
    // Get the host and the port
    let host = url.host().expect("uri has no host");
    let port = url.port_u16().unwrap_or(80);
    let address = format!("{}:{}", host, port);
    println!("50 {:?}", now.elapsed());
    const CONNECTION_TIME: u64 = 100;
    let stream = tokio::net::TcpStream::connect(address).await.unwrap();
    // let stream_result = match tokio::time::timeout(
    //     tokio::time::Duration::from_secs(5),
    //     tokio::net::TcpStream::connect(address), // 默认4s超时 花了3ms？？？why
    // )
    // .await
    // {
    //     Ok(ok) => ok,
    //     Err(e) => { // 连接失败了
    //         println!("connect timeout");
    //         panic!("fff")
    //     }
    // };

    // let stream = match stream_result {
    //     Ok(s) => s,
    //     Err(e) => {
    //         println!("connect timeout{}", e.kind());
    //         panic!("---")
    //     }
    // };
    println!("71 {:?}", now.elapsed());
    // .expect("Error while connecting to server");
    let io = TokioIo::new(stream);
    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await.unwrap();
    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            println!("Connection failed: {:?}", err);
        }
    });
    let authority = url.authority().unwrap().clone();

    // Create an HTTP request with an empty body and a HOST header
    let req = Request::builder()
        .method("GET")
        .uri( "/")//url)
        .header(hyper::header::HOST, authority.as_str())
        .body(Empty::<Bytes>::new())
        .unwrap();
    println!("89 {:?}", now.elapsed());
    // Await the response...
    let mut res = sender.send_request(req).await.unwrap();
    println!("92 {:?}", now.elapsed());
    if res.status() == StatusCode::OK {
        println!("Response status: {}", res.status());
    } else {
        println!("Error status: {} {:?}", res.status(), res);
    }
    println!("98 {:?}", now.elapsed());

    return Ok(())


}




async fn main1() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {

    // async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();
    let uri = opt.request_uri;
    let thread_num = opt.thread_num;
    let syn_timeout = opt.syn_timeout; // ms
    let request_timeout = opt.request_timeout; // ms
    let tps: u64 = opt.tps; //_000_000;
    let tps_per_thread = tps / thread_num;
    let mut handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();

    use tokio::sync::mpsc;
    let (count_sender, mut count_receiver) =
        mpsc::unbounded_channel::<(u8, tokio::time::Duration)>();
    let count_task = tokio::spawn(async move {
        let mut ticker = tokio::time::interval(tokio::time::Duration::from_millis(1000));
        let mut count = 0;
        let mut err_count = 0;
        let mut duration = tokio::time::Duration::from_millis(0);
        let mut max_duration = tokio::time::Duration::from_millis(0);
        let mut min_duration = tokio::time::Duration::from_millis(syn_timeout + request_timeout);
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    if count + err_count != 0 {
                        println!("ok: {} r/s;  err:  {} r/s avg: {:?} max: {:?} min: {:?}", count, err_count, duration / (count+err_count), max_duration, min_duration);

                    } else {
                        println!("ok: {} r/s;  err:  {} r/s", count, err_count);

                    }
                    count = 0;
                    err_count = 0;
                    duration = tokio::time::Duration::from_millis(0);
                    max_duration = tokio::time::Duration::from_millis(0);
                    min_duration = tokio::time::Duration::from_millis(syn_timeout + request_timeout);
                }
                val = count_receiver.recv() => {
                    match val.unwrap().0 {
                        1 => {
                            count += 1;
                            // println!("{}", val.unwrap());
                        }
                        _ => {
                            err_count += 1;
                        }
                    }
                    let d = val.unwrap().1;
                    if d > max_duration {
                        max_duration = d;
                    }
                    if d < min_duration {
                        min_duration = d;
                    }
                    duration += d;
                }
            }
        }
    });
    let interval_time =
        tokio::time::Duration::from_micros((1.0 / (tps_per_thread as f64) * 1000_000.0) as u64); // us
    println!("time {:?}", interval_time);
    for _x in 0..thread_num {
        let st = syn_timeout;
        let rt = request_timeout;
        let uri = uri.clone();
        let cs = count_sender.clone();
        let handle = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval_time);

            loop {
                // println!("->{} {:?}", x, );
                ticker.tick().await;
                let now = tokio::time::Instant::now();

                let url = uri.parse::<hyper::Uri>().unwrap();
                // Get the host and the port
                let host = url.host().expect("uri has no host");
                let port = url.port_u16().unwrap_or(80);
                let address = format!("{}:{}", host, port);
                // Open a TCP connection to the remote host
                let now = tokio::time::Instant::now();

                const CONNECTION_TIME: u64 = 100;
                let stream_result = match tokio::time::timeout(
                    tokio::time::Duration::from_secs(5),
                    tokio::net::TcpStream::connect(address), // 默认4s超时
                )
                .await
                {
                    Ok(ok) => ok,
                    Err(e) => { // 连接失败了
                        println!("connect timeout");
                        cs.send((0, now.elapsed())).unwrap();
                        continue;
                    }
                };

                let stream = match stream_result {
                    Ok(s) => s,
                    Err(e) => {
                        cs.send((0, now.elapsed())).unwrap();
                        // println!("connect timeout{}", e.kind());
                        continue;
                    }
                };
                // .expect("Error while connecting to server");
                let io = TokioIo::new(stream);
                let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await.unwrap();
                tokio::task::spawn(async move {
                    if let Err(err) = conn.await {
                        println!("Connection failed: {:?}", err);
                    }
                });
                let authority = url.authority().unwrap().clone();

                // Create an HTTP request with an empty body and a HOST header
                let req = Request::builder()
                .method("GET")
                    .uri(url)
                    .header(hyper::header::HOST, authority.as_str())
                    .body(Empty::<Bytes>::new())
                    .unwrap();

                // Await the response...
                let mut res = sender.send_request(req).await.unwrap();

                if res.status() == StatusCode::OK {
                    // println!("Response status: {}", res.status());
                    cs.send((1, now.elapsed())).unwrap();
                } else {
                    println!("Error status: {}", res.status());
                    cs.send((0, now.elapsed())).unwrap();
                }
            }
        });
        handles.push(handle);
    }
    for x in handles {
        x.await.unwrap();
    }
    count_task.await.unwrap();
    Ok(())
}

// [dependencies]
// reqwest = { version = "0.11", features = ["json"] }
// tokio = { version = "1", features = ["full"] }
