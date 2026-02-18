//! TUI Monitor — visualizes A2A tasks in real-time.
//!
//! This tool watches a directory of task JSON files (created by FileTaskStore)
//! and displays them in a terminal UI using ratatui, inspired by Claude Code's
//! task visualization.
//!
//! Usage:
//! ```sh
//! # Terminal 1: Start an agent with FileTaskStore
//! cargo run --example echo_agent_file
//!
//! # Terminal 2: Run the TUI monitor
//! cargo run --example tui_monitor -- ./tasks
//! ```
//!
//! Controls:
//! - `q` or `Ctrl+C`: Quit
//! - `r`: Refresh (force reload)

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

/// Task state enum (subset needed for visualization)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum TaskState {
    Submitted,
    Working,
    Completed,
    Failed,
    Canceled,
    InputRequired,
    Rejected,
    AuthenticationRequired,
}

impl TaskState {
    fn icon(&self) -> &str {
        match self {
            TaskState::Submitted => "⏸",
            TaskState::Working => "⚙",
            TaskState::Completed => "✓",
            TaskState::Failed => "✗",
            TaskState::Canceled => "⊗",
            TaskState::InputRequired => "?",
            TaskState::Rejected => "⊘",
            TaskState::AuthenticationRequired => "🔒",
        }
    }

    fn color(&self) -> Color {
        match self {
            TaskState::Submitted => Color::Yellow,
            TaskState::Working => Color::Cyan,
            TaskState::Completed => Color::Green,
            TaskState::Failed => Color::Red,
            TaskState::Canceled => Color::Gray,
            TaskState::InputRequired => Color::Magenta,
            TaskState::Rejected => Color::Red,
            TaskState::AuthenticationRequired => Color::Yellow,
        }
    }
}

/// Simplified task representation for display
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Task {
    id: String,
    context_id: String,
    status: TaskStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskStatus {
    state: TaskState,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<serde_json::Value>,
}

/// Application state
struct App {
    tasks_dir: PathBuf,
    tasks: HashMap<String, Task>,
    should_quit: bool,
    error_message: Option<String>,
}

impl App {
    fn new(tasks_dir: PathBuf) -> Self {
        Self {
            tasks_dir,
            tasks: HashMap::new(),
            should_quit: false,
            error_message: None,
        }
    }

    fn load_tasks(&mut self) -> Result<()> {
        let mut new_tasks = HashMap::new();

        if !self.tasks_dir.exists() {
            self.error_message = Some(format!("Directory does not exist: {:?}", self.tasks_dir));
            return Ok(());
        }

        for entry in std::fs::read_dir(&self.tasks_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            match std::fs::read_to_string(&path) {
                Ok(contents) => match serde_json::from_str::<Task>(&contents) {
                    Ok(task) => {
                        new_tasks.insert(task.id.clone(), task);
                    }
                    Err(e) => {
                        eprintln!("Failed to parse task file {:?}: {}", path, e);
                    }
                },
                Err(e) => {
                    eprintln!("Failed to read task file {:?}: {}", path, e);
                }
            }
        }

        self.tasks = new_tasks;
        self.error_message = None;
        Ok(())
    }

    fn render_ui(&self, frame: &mut ratatui::Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(frame.area());

        // Title
        let title = Paragraph::new(format!("A2A Task Monitor - {}", self.tasks_dir.display()))
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(title, chunks[0]);

        // Task list
        let items: Vec<ListItem> = {
            let mut task_list: Vec<(&String, &Task)> = self.tasks.iter().collect();
            // Sort by task ID for consistent display
            task_list.sort_by_key(|(id, _)| *id);

            task_list
                .iter()
                .map(|(_, task)| {
                    let state = &task.status.state;
                    let icon = state.icon();
                    let color = state.color();
                    
                    let content = vec![Line::from(vec![
                        Span::styled(format!("{} ", icon), Style::default().fg(color).add_modifier(Modifier::BOLD)),
                        Span::styled(&task.id, Style::default().fg(Color::White)),
                        Span::raw(" "),
                        Span::styled(format!("[{:?}]", state), Style::default().fg(color)),
                    ])];

                    ListItem::new(content)
                })
                .collect()
        };

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Tasks"));
        frame.render_widget(list, chunks[1]);

        // Status bar
        let status_text = if let Some(ref err) = self.error_message {
            format!("Error: {} | q: quit | r: refresh", err)
        } else {
            format!("Tasks: {} | q: quit | r: refresh", self.tasks.len())
        };

        let status = Paragraph::new(status_text)
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Left)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(status, chunks[2]);
    }
}

fn main() -> Result<()> {
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let tasks_dir = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        eprintln!("Usage: {} <tasks_directory>", args[0]);
        eprintln!("\nExample:");
        eprintln!("  {} ./tasks", args[0]);
        std::process::exit(1);
    };

    // Initialize the app
    let mut app = App::new(tasks_dir.clone());
    app.load_tasks()?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Setup file watcher
    let (tx, rx) = mpsc::channel();
    let tx_clone = tx.clone();
    let mut watcher = RecommendedWatcher::new(
        move |res| {
            tx_clone.send(res).unwrap();
        },
        Config::default(),
    )?;
    watcher.watch(&tasks_dir, RecursiveMode::NonRecursive)?;

    // Main loop
    let result = run_app(&mut terminal, &mut app, rx);

    // Cleanup
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    rx: mpsc::Receiver<notify::Result<notify::Event>>,
) -> Result<()> {
    loop {
        terminal.draw(|f| app.render_ui(f))?;

        // Check for file system events (non-blocking)
        if let Ok(Ok(_event)) = rx.try_recv() {
            // Reload tasks on any change
            if let Err(e) = app.load_tasks() {
                app.error_message = Some(format!("Failed to reload tasks: {}", e));
            }
        }

        // Check for keyboard events
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        app.should_quit = true;
                    }
                    KeyCode::Char('r') => {
                        if let Err(e) = app.load_tasks() {
                            app.error_message = Some(format!("Failed to reload tasks: {}", e));
                        }
                    }
                    KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        app.should_quit = true;
                    }
                    _ => {}
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
