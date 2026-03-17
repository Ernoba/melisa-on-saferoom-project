use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;
use std::future::Future;

// Fungsi ini menerima pesan dan sebuah 'closure' (tugas yang ingin dijalankan)
pub async fn execute_with_spinner<F, T>(message: &str, future: F) -> T 
where 
    F: Future<Output = T> 
{
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            // Tambahin {elapsed_precise} biar keren kayak tool CLI pro
            .template("{spinner:.green} [{elapsed_precise}] {msg}") 
            .unwrap(),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));

    // 3. Jalankan tugas asinkronnya dengan .await di sini
    let result = future.await;

    // Hentikan spinner setelah tugas selesai
    pb.finish_and_clear();
    
    result
}