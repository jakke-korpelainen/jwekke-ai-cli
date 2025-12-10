use crate::{client::list_mistral_models, config::save_model_name, logger::Logger, ui};
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use inquire::{InquireError, Select};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::io::{self, Stdout};
use tokio::sync::mpsc;

/// Displays a list of available Mistral AI models and prompts the user to select one.
///
/// # Returns
/// A `Result` containing the selected model's name or an error.
pub async fn select_mistral_model(logger: &Logger) -> Result<String, Box<dyn std::error::Error>> {
    let models = list_mistral_models(&logger).await.map_err(|e| {
        _ = ui::restore_terminal();
        panic!("Failed to load models: {}", e);
    })?;

    let options = models
        .iter()
        .map(|model| model.id.as_str())
        .collect::<Vec<&str>>();

    let ans: Result<&str, InquireError> = Select::new("Select Mistral Model", options)
        .with_page_size(15)
        .prompt();

    let selection = match ans {
        Ok(choice) => choice.to_string(),
        Err(_) => {
            logger.log_error(format!("Error selecting model")).await;
            return Err("Selection failed".into());
        }
    };

    _ = save_model_name(selection.clone()).await.map_err(async |e| {
        logger
            .log_error(format!("Failed to save model to config. {}", e))
            .await;
        e
    });

    Ok(selection)
}

pub fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    Terminal::new(CrosstermBackend::new(stdout))
}

pub fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(
        io::stdout(),
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
    )?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}

pub async fn render_ui(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    logger: &Logger,
    model: String,
    prompt: String,
    mut token_receiver: mpsc::Receiver<String>,
) -> io::Result<()> {
    let errors = logger.get_errors().await;
    let mut token_stream = String::new();
    let mut should_quit = false;
    let mut scroll_offset = 0;

    enable_raw_mode()?;

    execute!(
        io::stdout(),
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
    )?;

    while !should_quit {
        if crossterm::event::poll(std::time::Duration::from_millis(100))? {
            if let crossterm::event::Event::Key(key_event) = crossterm::event::read()? {
                match key_event.code {
                    crossterm::event::KeyCode::Char('q') => {
                        should_quit = true;
                    }
                    crossterm::event::KeyCode::Up => {
                        if scroll_offset > 0 {
                            scroll_offset -= 1;
                        }
                    }
                    crossterm::event::KeyCode::Down => {
                        scroll_offset += 1;
                    }
                    _ => {}
                }
            }
        }

        // Handle token stream updates
        if let Ok(token) = token_receiver.try_recv() {
            token_stream.push_str(&token);
        }

        terminal.draw(|f| {
            let size = f.size();
            let constraints: Vec<Constraint> = if errors.is_empty() {
                vec![
                    Constraint::Length(5),
                    Constraint::Min(1),
                    Constraint::Length(3),
                ]
            } else {
                vec![
                    Constraint::Length(5),
                    Constraint::Length(5),
                    Constraint::Min(1),
                    Constraint::Length(3),
                ]
            };

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(constraints.clone())
                .split(size);

            if !errors.is_empty() {
                let errors_text: Vec<Line> = errors
                    .iter()
                    .map(|error| {
                        Line::from(Span::styled(
                            error.as_str(),
                            Style::default().fg(Color::Red),
                        ))
                    })
                    .collect();
                let errors_paragraph = Paragraph::new(errors_text)
                    .block(Block::default().borders(Borders::ALL).title("Errors"));
                f.render_widget(errors_paragraph, chunks[0]);
            }

            let model_prompt_index = if errors.is_empty() { 0 } else { 1 };
            let model_prompt_text = vec![
                Line::from(vec![
                    Span::styled("Model: ", Style::default().fg(Color::Yellow)),
                    Span::styled(model.as_str(), Style::default().fg(Color::Green)),
                ]),
                Line::from(vec![
                    Span::styled("Prompt: ", Style::default().fg(Color::Yellow)),
                    Span::styled(prompt.as_str(), Style::default().fg(Color::Green)),
                ]),
            ];
            let model_prompt_paragraph = Paragraph::new(model_prompt_text).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Model and Prompt"),
            );
            f.render_widget(model_prompt_paragraph, chunks[model_prompt_index]);

            let token_stream_index = if errors.is_empty() { 1 } else { 2 };
            let token_stream_paragraph = Paragraph::new(token_stream.clone())
                .block(Block::default().borders(Borders::ALL).title("Token Stream"))
                .wrap(Wrap { trim: true }) // Enable word wrapping
                .scroll((scroll_offset as u16, 0)); // Apply scroll offset
            f.render_widget(token_stream_paragraph, chunks[token_stream_index]);
            let controls_index = if errors.is_empty() { 2 } else { 3 };
            let controls_text = Line::from(vec![
                Span::styled("Press ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    "q",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                ),
                Span::styled(" to quit. Use ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    "↑",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                ),
                Span::styled("/", Style::default().fg(Color::Yellow)),
                Span::styled(
                    "↓",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                ),
                Span::styled(" to scroll.", Style::default().fg(Color::Yellow)),
            ]);
            let controls_paragraph = Paragraph::new(controls_text)
                .block(Block::default().borders(Borders::ALL).title("Controls"));
            f.render_widget(controls_paragraph, chunks[controls_index]);
        })?;
    }

    disable_raw_mode()?;
    Ok(())
}
