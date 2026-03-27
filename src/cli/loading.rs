use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;
use std::future::Future;

/// Executes an asynchronous task (Future) while displaying a highly responsive,
/// non-blocking terminal spinner. This provides enterprise-grade UX by preventing
/// the terminal from appearing "frozen" during long-running network or I/O operations.
///
/// # Arguments
/// * `message`      - The informational text to display alongside the loading spinner.
/// * `task_builder` - A closure that takes a clone of the ProgressBar and returns the
///                    asynchronous task to execute.
/// * `audit`        - When true the spinner is completely suppressed so that raw
///                    subprocess output (already going to stdout/stderr via
///                    Stdio::inherit) is not interleaved with spinner control codes.
pub async fn execute_with_spinner<C, F, T>(message: &str, task_builder: C, audit: bool) -> T
where
    C: FnOnce(ProgressBar) -> F,
    F: Future<Output = T>,
{
    // ── AUDIT MODE ────────────────────────────────────────────────────────────
    // Ketika audit aktif, spinner TIDAK ditampilkan sama sekali.
    // Kita tetap membuat ProgressBar tetapi langsung menyelesaikannya (hidden),
    // sehingga semua pb.println() di dalam task menjadi println! biasa dan
    // output subprocess yang sudah di-Stdio::inherit langsung mengalir ke terminal
    // tanpa konflik dengan animasi spinner.
    if audit {
        let pb = ProgressBar::hidden();
        let result = task_builder(pb.clone()).await;
        pb.finish_and_clear();
        return result;
    }

    // ── NORMAL MODE ───────────────────────────────────────────────────────────
    // 1. Initialize a new dynamic spinner instance
    let pb = ProgressBar::new_spinner();

    // 2. Configure the visual style and animation sequence of the spinner.
    //    Template: [Spinner] Message   (elapsed_precise dihapus — penyebab jejak timestamp)
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            // PERUBAHAN: {elapsed_precise} DIHAPUS dari template.
            // Inilah yang menyebabkan tanda waktu [00:00:06] tertinggal sebagai
            // "jejak" di layar setiap kali println! menulis baris baru di tengah
            // animasi spinner. Tanpa timestamp, spinner hanya menulis satu baris
            // karakter yang selalu ditimpa di posisi yang sama.
            .template("{spinner:.green.bold} {msg:.white.bold}")
            .expect("FATAL: Failed to parse indicatif progress template string"),
    );

    // 3. Bind the execution message and set the background animation tick rate.
    pb.set_message(message.to_string());
    // 80ms yields a smoother 12.5 FPS animation compared to the standard 100ms.
    pb.enable_steady_tick(Duration::from_millis(80));

    // 4. Await the actual workload payload.
    //    The spinner continues to tick on an isolated background thread managed by
    //    indicatif while Tokio handles the async future here.
    //    We inject a cloned ProgressBar into the closure so the caller can safely
    //    print messages using `pb.println()` without breaking the animation.
    let result = task_builder(pb.clone()).await;

    // 5. Cleanup Protocol: wipe the spinner from the terminal line once done.
    pb.finish_and_clear();

    // 6. Return the evaluated output of the asynchronous task back to the caller.
    result
}