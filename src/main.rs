use core::time;

use reqwest::{ClientBuilder, StatusCode};
use structopt::StructOpt;




#[derive(Debug, StructOpt, Clone)]
#[structopt(name = "tcp-listen", about = "An example of StructOpt usage.")]
struct Opt {
    /// Activate debug mode
    // short and long flags (-d, --debug) will be deduced from the field's name
    #[structopt(short = "q", long = "qps", default_value = "1000")]
    tps: u64,

    // request sleep (ms)
    #[structopt(short = "t", long="thread_num", default_value = "0")]
    thread_num: u64,

    // request level tcp or http (ms)
    #[structopt(short = "st", long="syntm", default_value = "6000")]
    syn_timeout: u64,

    // request level tcp or http (ms)
    #[structopt(short = "rt", long="reqtm", default_value = "6000")]
    request_timeout: u64,
}



#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();
    let uri = "http://localhost:8080/";
    let thread_num = 1u64;
    let syn_timeout = 6000; // ms
    let request_timeout = 6000; // ms
    let tps: u64 = 10; //_000_000;
    let tps_per_thread = tps / thread_num;
    let mut handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();

    use tokio::sync::mpsc;
    let (count_sender, mut count_receiver) = mpsc::unbounded_channel::<(u8, tokio::time::Duration)>();
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
    println!("time{:?}", interval_time);
    for _x in 0..thread_num {
        let st = syn_timeout;
        let rt = request_timeout;
        let uri = uri;
        let cs = count_sender.clone();
        let handle = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval_time);

            loop {
                // println!("->{} {:?}", x, );
                ticker.tick().await;
                let now = tokio::time::Instant::now();
                let client = ClientBuilder::new()
                .connect_timeout(std::time::Duration::from_millis(st))
                .timeout(std::time::Duration::from_millis(rt))
                .tcp_nodelay(false)
                .build()
                .unwrap();
                // println!("{:?}[new]", now.elapsed());

                // let now = tokio::time::Instant::now();
                if let Ok(resp) = client.get(uri).send().await {
                    if resp.status().is_success() {
                        // println!("{:?}[OK]", now.elapsed());

                        cs.send((1, now.elapsed())).unwrap();
                    }
                } else {
                    // println!("{}[TM]", x);
                    cs.send((0, now.elapsed())).unwrap();
                }
                // cs.send((0, now.elapsed())).unwrap();
                // drop(client);
                // println!("<-{} {:?}", x, tokio::time::Instant::now());
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