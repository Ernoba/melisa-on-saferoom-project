use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;
use std::future::Future;

/// Executes an asynchronous task (Future) while displaying a highly responsive, 
/// non-blocking terminal spinner. This provides enterprise-grade UX by preventing 
/// the terminal from appearing "frozen" during long-running network or I/O operations.
///
/// # Arguments
/// * `message` - The informational text to display alongside the loading spinner.
/// * `task_builder` - A closure that takes a clone of the ProgressBar and returns the asynchronous task to execute.
pub async fn execute_with_spinner<C, F, T>(message: &str, task_builder: C) -> T 
where 
    C: FnOnce(ProgressBar) -> F,
    F: Future<Output = T> 
{
    // 1. Initialize a new dynamic spinner instance
    let pb = ProgressBar::new_spinner();
    
    // 2. Configure the visual style and animation sequence of the spinner
    pb.set_style(
        ProgressStyle::default_spinner()
            // Utilizing a smooth, high-resolution dot animation sequence
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            // Template mapping: [Spinner] [Elapsed Time] Message
            // Added advanced color coding: Bold Green Spinner, Cyan Timer, Bold White Text
            .template("{spinner:.green.bold} [{elapsed_precise:.cyan}] {msg:.white.bold}") 
            .expect("FATAL: Failed to parse indicatif progress template string")
    );
    
    // 3. Bind the execution message and set the background animation tick rate
    pb.set_message(message.to_string());
    // 80ms yields a smoother 12.5 FPS animation compared to the standard 100ms
    pb.enable_steady_tick(Duration::from_millis(80)); 

    // 4. Await the actual workload payload
    // Note: The spinner continues to tick gracefully on an isolated background thread 
    // managed by the indicatif library while Tokio handles the async future here.
    // We inject a cloned instance of the ProgressBar into the closure so the caller 
    // can safely print messages using `pb.println()` without breaking the animation.
    let result = task_builder(pb.clone()).await;

    // 5. Cleanup Protocol: Wipe the spinner from the terminal line once the task completes.
    // This ensures the terminal standard output remains pristine for the next command sequence.
    pb.finish_and_clear();
    
    // 6. Return the evaluated output of the asynchronous task back to the caller
    result
}