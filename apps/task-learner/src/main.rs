use shared::core::config::AppConfig;

mod collector;
mod learner;
mod persistence;

fn main() -> Result<(), anyhow::Error> {
    // 1. Load Configuration
    println!("Loading configuration...");
    let config = AppConfig::load()?;
    println!("Configuration loaded successfully.");
    println!("Event log directory: {}", config.event_log_dir.display());
    println!("Learned tasks directory: {}", config.learned_tasks_dir.display());

    // 2. Collect Events
    println!("\nCollecting events from the latest session...");
    let events = match collector::load_latest_session_events(&config) {
        Ok(events) => {
            if events.is_empty() {
                println!("No events found in the latest session. Exiting.");
                return Ok(());
            }
            println!("Successfully loaded {} events.", events.len());
            events
        }
        Err(e) => {
            eprintln!("Failed to load events: {}", e);
            eprintln!("Please ensure the user-monitor has been run and has recorded some events.");
            return Err(e);
        }
    };

    // Try to load DOM log lines for the same session for context
    let dom_lines = if let Some(session_id) = events.first().map(|e| e.session_id) {
        match collector::load_dom_events_for_session(&config, &session_id) {
            Ok(lines) => {
                if !lines.is_empty() { println!("Loaded {} DOM lines.", lines.len()); }
                lines
            }
            Err(_) => Vec::new(),
        }
    } else { Vec::new() };

    // 3. Learn Task
    println!("\nAnalyzing events to learn a new task...");
    if let Some(learned_task) = learner::learn_task_from_events_with_dom(&events, &dom_lines) {
        println!("A new task was learned successfully!");
        println!("  Task ID: {}", learned_task.id);
        println!("  Task Name: {}", learned_task.name);
        println!("  Number of steps: {}", learned_task.steps.len());

        // 4. Persist Task
        println!("\nSaving the learned task...");
        if let Err(e) = persistence::save_learned_task(&learned_task, &config) {
            eprintln!("Failed to save the learned task: {}", e);
            return Err(e);
        }
        println!("Task has been saved successfully.");
    } else {
        println!("\nNo repetitive task pattern was found in the latest session.");
    }

    Ok(())
}
