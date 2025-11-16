use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use crate::model::PromptRecord;

pub fn original_lines(record: Option<&PromptRecord>) -> Vec<Line<'static>> {
    match record {
        Some(prompt) => prompt_theme_lines(&prompt.raw_text),
        None => vec![Line::from("Select a prompt to view the full text.")],
    }
}

pub fn review_lines(record: Option<&PromptRecord>) -> Vec<Line<'static>> {
    match record {
        Some(prompt) => match prompt.latest_review.as_ref() {
            Some(review) => {
                let mut lines = Vec::new();
                lines.extend(prompt_theme_lines(&review.improved_prompt));

                if !review.explanation.trim().is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "Explanation:",
                        ratatui::style::Style::default()
                            .add_modifier(ratatui::style::Modifier::BOLD),
                    )));
                    lines.push(Line::from(""));
                    for line in review.explanation.lines() {
                        if line.is_empty() {
                            lines.push(Line::from(""));
                        } else {
                            lines.push(Line::from(line.to_string()));
                        }
                    }
                }

                // Add tags if present
                if !review.tags.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(format!("Tags: {}", review.tags.join(", "))));
                }

                lines
            }
            None => vec![Line::from(
                "No improved prompt yet. Press `r` to request a Sage review.",
            )],
        },
        None => vec![Line::from(
            "Improved prompt will appear here after selecting a prompt.",
        )],
    }
}

fn prompt_theme_lines(text: &str) -> Vec<Line<'static>> {
    if text.trim().is_empty() {
        return vec![Line::from(Span::styled(
            "∅ Empty prompt",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        ))];
    }

    let accent = Style::default().fg(Color::Rgb(255, 214, 194));
    let body = Style::default()
        .fg(Color::Rgb(210, 212, 230))
        .bg(Color::Rgb(31, 32, 40));

    text.lines()
        .map(|line| {
            Line::from(vec![
                Span::styled("▌ ", accent),
                Span::styled(line.to_string(), body),
            ])
        })
        .collect()
}
