use osc94::Progress;
use std::io::Result;
use tokio::time::{Duration, sleep};

const DURATION: Duration = Duration::from_millis(50);
const PROGRESS_BAR_CHAR: char = '█';

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    test_progress().await?;

    Ok(())
}

async fn test_progress() -> Result<()> {
    let mut progress = Progress::default();
    progress.start();

    for _ in 0..=100 {
        sleep(DURATION).await;
        // Progress bar sequences
        progress.increment(1).flush()?;
        // Progress bar
        let width = progress.get_progress() as usize * 50 / 100;
        let bar = PROGRESS_BAR_CHAR.to_string().repeat(width);
        eprint!("[{:50}] {}%\r", bar, progress.get_progress());
        // Terminal title
        title(&format!("Progress: {}%", progress.get_progress()));
    }
    eprintln!();

    Ok(())
}

fn title(s: &str) {
    eprint!("\x1b]0;{s}\x07");
}
