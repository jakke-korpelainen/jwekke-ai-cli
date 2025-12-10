use crate::{client::list_mistral_models, config::save_model_name, logger::Logger};
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
    widgets::{Block, Borders, Paragraph},
};
use std::io::{self, Stdout};
use tokio::sync::mpsc;

/// Displays a list of available Mistral AI models and prompts the user to select one.
///
/// # Returns
/// A `Result` containing the selected model's name or an error.
pub async fn select_mistral_model(logger: &Logger) -> Result<String, Box<dyn std::error::Error>> {
    let models = list_mistral_models(&logger).await.map_err(|e| {
        eprintln!("Failed to load models: {}", e);
        e
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

/// Initializes the terminal for the UI.
pub fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    Terminal::new(CrosstermBackend::new(stdout))
}

/// Restores the terminal to its original state.
pub fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}

/// Renders the UI with the given errors, model, prompt, and token stream.
pub async fn render_ui(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    logger: &Logger,
    model: String,
    prompt: String,
    mut token_receiver: mpsc::Receiver<String>,
) -> io::Result<()> {
    let errors = logger.get_errors().await;

    terminal.draw(|f| {
        let size = f.size();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(5), // Errors section
                    Constraint::Length(5), // Model and Prompt section
                    Constraint::Min(1),    // Token stream section
                ]
                .as_ref(),
            )
            .split(size);

        // Errors section
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

        // Model and Prompt section
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
        f.render_widget(model_prompt_paragraph, chunks[1]);

        // Token stream section
        let token_stream_paragraph =
            Paragraph::new("").block(Block::default().borders(Borders::ALL).title("Token Stream"));
        f.render_widget(token_stream_paragraph, chunks[2]);
    })?;

    // Handle token stream updates
    let mut token_stream = String::new();
    while let Some(token) = token_receiver.recv().await {
        token_stream.push_str(&token);
        terminal.draw(|f| {
            let size = f.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Length(5), // Errors section
                        Constraint::Length(5), // Model and Prompt section
                        Constraint::Min(1),    // Token stream section
                    ]
                    .as_ref(),
                )
                .split(size);

            let token_stream_paragraph = Paragraph::new(token_stream.clone())
                .block(Block::default().borders(Borders::ALL).title("Token Stream"));
            f.render_widget(token_stream_paragraph, chunks[2]);
        })?;
    }

    Ok(())
}
