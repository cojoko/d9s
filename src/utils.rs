use chrono::{TimeZone, Utc};

pub fn get_status_style(status: &str) -> ratatui::style::Style {
    use ratatui::style::Color;

    match status.trim_matches('"') {
        "SUCCESS" => ratatui::style::Style::default().fg(Color::Green),
        "FAILURE" => ratatui::style::Style::default().fg(Color::Red),
        "STARTED" | "STARTING" => ratatui::style::Style::default().fg(Color::Blue),
        "QUEUED" => ratatui::style::Style::default().fg(Color::Yellow),
        "CANCELED" => ratatui::style::Style::default().fg(Color::DarkGray),
        _ => ratatui::style::Style::default(),
    }
}

pub fn format_timestamp(timestamp: Option<f64>) -> String {
    timestamp.map_or("-".to_string(), |ts| {
        let utc_time = Utc.timestamp_opt(ts as i64, 0).single().unwrap_or_default();
        let now = Utc::now();
        let duration = now.signed_duration_since(utc_time);

        // If started within the last 24 hours, show relative time
        if duration.num_seconds() >= 0 && duration.num_hours() < 24 {
            if duration.num_hours() > 0 {
                format!("{} hour(s) ago", duration.num_hours())
            } else if duration.num_minutes() > 0 {
                format!("{} minute(s) ago", duration.num_minutes())
            } else {
                format!("{} second(s) ago", duration.num_seconds().max(1)) // At least 1 second
            }
        } else {
            // Otherwise show the date and time
            utc_time.format("%Y-%m-%d %H:%M:%S").to_string()
        }
    })
}

pub fn format_full_timestamp(timestamp: Option<f64>) -> String {
    timestamp.map_or_else(
        || "Not started".to_string(),
        |ts| {
            let utc_time = Utc.timestamp_opt(ts as i64, 0).single().unwrap_or_default();
            let local_tz = chrono::Local::now().timezone();
            let local_time = utc_time.with_timezone(&local_tz);
            let duration = Utc::now().signed_duration_since(utc_time);
            let ago = if duration.num_days() > 0 {
                format!("{} day(s) ago", duration.num_days())
            } else if duration.num_hours() > 0 {
                format!("{} hour(s) ago", duration.num_hours())
            } else if duration.num_minutes() > 0 {
                format!("{} minute(s) ago", duration.num_minutes())
            } else {
                format!("{} second(s) ago", duration.num_seconds())
            };
            format!(
                "{} ({} - {})",
                local_time.format("%Y-%m-%d %H:%M:%S %Z"),
                utc_time.format("%H:%M:%S UTC"),
                ago
            )
        },
    )
}

pub fn format_duration(start: Option<f64>, end: Option<f64>) -> String {
    match (start, end) {
        (Some(start_time), Some(end_time)) => {
            let duration = end_time - start_time;
            if duration < 60.0 {
                format!("{:.1}s", duration)
            } else if duration < 3600.0 {
                format!("{:.1}m", duration / 60.0)
            } else {
                format!("{:.1}h", duration / 3600.0)
            }
        }
        (Some(_), None) => "Running".to_string(),
        _ => "-".to_string(),
    }
}

pub fn truncate(s: &str, max_width: usize) -> String {
    if s.len() <= max_width {
        s.to_string()
    } else {
        format!("{}...", &s[0..max_width - 3])
    }
}
