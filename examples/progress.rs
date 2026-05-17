use osc94::Progress;
use std::io::Result;
use tokio::time::{Duration, sleep};

const DURATION: Duration = Duration::from_millis(50);

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let mut progress = Progress::default();
    progress.start();

    for _ in 0..=100 {
        work().await;
        progress.increment(1).flush()?;
    }

    Ok(())
}

async fn work() {
    sleep(DURATION).await;
}
