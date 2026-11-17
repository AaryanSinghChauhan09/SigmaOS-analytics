//! Event aggregation and analytics for SigmaOS.
//!
//! Dependency-free event collection, counters, histograms, and persistence.
//! Useful for monitoring usage, performance, and error tracking.

use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::Path;

/// A single analytics event.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub timestamp: i64,
    pub category: String,
    pub action: String,
    pub value: Option<f64>,
    pub tags: Vec<String>,
}

impl Event {
    pub fn new(category: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            category: category.into(),
            action: action.into(),
            value: None,
            tags: Vec::new(),
        }
    }

    pub fn with_value(mut self, value: f64) -> Self {
        self.value = Some(value);
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
}

/// Errors for analytics operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnalyticsError {
    Io(String),
    Format(String),
}

impl fmt::Display for AnalyticsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::Format(e) => write!(f, "format error: {e}"),
        }
    }
}

impl std::error::Error for AnalyticsError {}

const MAGIC: &str = "SIGANAL1";

/// In-memory analytics store with aggregation.
#[derive(Debug, Clone, PartialEq)]
pub struct Analytics {
    events: Vec<Event>,
    counters: HashMap<String, i64>,
    sums: HashMap<String, f64>,
}

impl Analytics {
    pub fn new() -> Self {
        Self { events: Vec::new(), counters: HashMap::new(), sums: HashMap::new() }
    }

    pub fn record(&mut self, event: Event) {
        let key = format!("{}:{}", event.category, event.action);
        *self.counters.entry(key.clone()).or_insert(0) += 1;
        if let Some(v) = event.value {
            *self.sums.entry(key).or_insert(0.0) += v;
        }
        self.events.push(event);
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    pub fn get_counter(&self, category: &str, action: &str) -> i64 {
        let key = format!("{}:{}", category, action);
        *self.counters.get(&key).unwrap_or(&0)
    }

    pub fn get_sum(&self, category: &str, action: &str) -> f64 {
        let key = format!("{}:{}", category, action);
        *self.sums.get(&key).unwrap_or(&0.0)
    }

    pub fn count_by_category(&self, category: &str) -> i64 {
        self.counters.iter()
            .filter(|(k, _)| k.starts_with(&format!("{}:", category)))
            .map(|(_, v)| *v)
            .sum()
    }

    pub fn histogram(&self, category: &str, action: &str, bins: usize) -> Vec<(f64, f64, usize)> {
        let values: Vec<f64> = self.events.iter()
            .filter(|e| e.category == category && e.action == action)
            .filter_map(|e| e.value)
            .collect();
        if values.is_empty() {
            return Vec::new();
        }
        let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = max - min;
        let bin_width = if range > 0.0 { range / bins as f64 } else { 1.0 };
        let mut counts = vec![0usize; bins];
        for v in values {
            let idx = if range > 0.0 {
                ((v - min) / bin_width).floor() as usize
            } else {
                0
            };
            let idx = if idx >= bins { bins - 1 } else { idx };
            counts[idx] += 1;
        }
        counts.iter().enumerate()
            .map(|(i, c)| (min + i as f64 * bin_width, min + (i + 1) as f64 * bin_width, *c))
            .collect()
    }

    /// Serialize to the line-based persistence format.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), AnalyticsError> {
        let mut out = String::new();
        out.push_str(MAGIC);
        out.push('\n');
        for e in &self.events {
            out.push_str(&format!(
                "EVENT|{}|{}|{}|{}|{}\n",
                e.timestamp,
                encode_field(&e.category),
                encode_field(&e.action),
                e.value.map(|v| v.to_string()).unwrap_or_default(),
                e.tags.iter().map(|t| encode_field(t)).collect::<Vec<_>>().join(",")
            ));
        }
        fs::write(path, out).map_err(|e| AnalyticsError::Io(e.to_string()))
    }

    /// Load analytics saved with [`Analytics::save`].
    pub fn load(path: impl AsRef<Path>) -> Result<Self, AnalyticsError> {
        let text = fs::read_to_string(path).map_err(|e| AnalyticsError::Io(e.to_string()))?;
        let mut lines = text.lines();
        if lines.next() != Some(MAGIC) {
            return Err(AnalyticsError::Format("bad magic".to_string()));
        }
        let mut analytics = Self::new();
        for line in lines {
            if let Some(rest) = line.strip_prefix("EVENT|") {
                let mut parts = rest.split('|');
                let _ts = parts.next().unwrap_or("0");
                let category = decode_field(parts.next().unwrap_or(""));
                let action = decode_field(parts.next().unwrap_or(""));
                let value = parts.next().and_then(|v| v.parse::<f64>().ok());
                let tags: Vec<String> = parts.next().unwrap_or("")
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(decode_field)
                    .collect();
                let mut event = Event::new(category, action);
                if let Some(v) = value {
                    event = event.with_value(v);
                }
                for tag in tags {
                    event = event.with_tag(tag);
                }
                analytics.record(event);
            }
        }
        Ok(analytics)
    }
}

fn encode_field(s: &str) -> String {
    s.replace('\\', "\\\\").replace('|', "\\p").replace('\n', "\\n")
}

fn decode_field(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('p') => out.push('|'),
                Some('\\') => out.push('\\'),
                Some(other) => { out.push('\\'); out.push(other); }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_and_counters_aaa() {
        // Arrange
        let mut a = Analytics::new();

        // Act
        a.record(Event::new("page", "view").with_tag("home"));
        a.record(Event::new("page", "view").with_tag("about"));
        a.record(Event::new("click", "button").with_value(1.0));

        // Assert
        assert_eq!(a.event_count(), 3);
        assert_eq!(a.get_counter("page", "view"), 2);
        assert_eq!(a.get_counter("click", "button"), 1);
        assert_eq!(a.count_by_category("page"), 2);
    }

    #[test]
    fn test_sums_and_histogram_aaa() {
        // Arrange
        let mut a = Analytics::new();

        // Act
        a.record(Event::new("latency", "ms").with_value(10.0));
        a.record(Event::new("latency", "ms").with_value(20.0));
        a.record(Event::new("latency", "ms").with_value(30.0));

        // Assert
        assert_eq!(a.get_sum("latency", "ms"), 60.0);
        let hist = a.histogram("latency", "ms", 3);
        assert_eq!(hist.len(), 3);
        assert_eq!(hist.iter().map(|(_, _, c)| c).sum::<usize>(), 3);
    }

    #[test]
    fn test_save_load_roundtrip_aaa() {
        // Arrange
        let mut a = Analytics::new();
        a.record(Event::new("test", "event").with_tag("a|b").with_value(1.5));
        let dir = std::env::temp_dir().join(format!("analytics-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("analytics.siganal");

        // Act
        a.save(&path).unwrap();
        let loaded = Analytics::load(&path).unwrap();

        // Assert
        assert_eq!(loaded.event_count(), 1);
        assert_eq!(loaded.get_counter("test", "event"), 1);
        assert_eq!(loaded.get_sum("test", "event"), 1.5);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
