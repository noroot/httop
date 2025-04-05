use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use chrono::prelude::*;
use regex::Regex;
use std::sync::mpsc;
use std::fs::File;

#[derive(Debug, Clone)]
struct Request {
    timestamp: DateTime<Utc>,
    ip: String,
    method: String,
    path: String,
    status_code: u16,
    response_time: f64,
    user_agent: String,
    bytes_sent: usize,
}

#[derive(Debug, Clone)]
struct Stats {
    total_requests: usize,
    requests_per_second: f64,
    bytes_sent: usize,
    status_codes: HashMap<u16, usize>,
    paths: HashMap<String, usize>,
    ips: HashMap<String, usize>,
    methods: HashMap<String, usize>,
    recent_requests: Vec<Request>,
}


enum SortBy {
    Count,
    Path,
    StatusCode,
    IP,
    UserAgent,
}

enum Command {
    Sort(SortBy),
    IncreaseLimit,
    DecreaseLimit,
    Quit,
    Noop,
}

struct Httop {
    stats: Arc<Mutex<Stats>>,
    sort_by: SortBy,
    display_limit: usize,
}

impl Stats {
    fn new() -> Self {
        Stats {
            total_requests: 0,
            requests_per_second: 0.0,
            bytes_sent: 0,
            status_codes: HashMap::new(),
            paths: HashMap::new(),
            ips: HashMap::new(),
            methods: HashMap::new(),
            recent_requests: Vec::new(),
        }
    }

    fn update(&mut self, request: Request) {
        self.total_requests += 1;
        self.bytes_sent += request.bytes_sent;

        *self.status_codes.entry(request.status_code).or_insert(0) += 1;
        *self.paths.entry(request.path.clone()).or_insert(0) += 1;
        *self.ips.entry(request.ip.clone()).or_insert(0) += 1;
        *self.methods.entry(request.method.clone()).or_insert(0) += 1;

        // Keep only the 100 most recent requests
        self.recent_requests.push(request);
        if self.recent_requests.len() > 100 {
            self.recent_requests.remove(0);
        }
    }
}

fn parse_log_line(line: &str) -> Option<Request> {
    // Common Nginx log format regex
    // Example: 192.168.1.1 - - [29/Nov/2021:12:34:56 +0000] "GET /page.html HTTP/1.1" 200 2326 "http://referrer.com" "Mozilla/5.0 ..." 0.002
    let re = Regex::new(r#"(\S+) (?:\S+) (?:\S+) \[([^\]]+)\] "(\S+) (\S+)[^"]+" (\d+) (\d+) "([^"]*)" "([^"]*)" (?:(\d+\.\d+))?"#).ok()?;

    let caps = re.captures(line)?;

    let timestamp_str = caps.get(2)?.as_str();
    let timestamp = DateTime::parse_from_str(timestamp_str, "%d/%b/%Y:%H:%M:%S %z")
        .ok()?
        .with_timezone(&Utc);

    let response_time = caps.get(9)
        .map_or(0.0, |m| m.as_str().parse::<f64>().unwrap_or(0.0));

    Some(Request {
        timestamp,
        ip: caps.get(1)?.as_str().to_string(),
        method: caps.get(3)?.as_str().to_string(),
        path: caps.get(4)?.as_str().to_string(),
        status_code: caps.get(5)?.as_str().parse().ok()?,
        bytes_sent: caps.get(6)?.as_str().parse().ok()?,
        user_agent: caps.get(8)?.as_str().to_string(),
        response_time,
    })
}

impl Httop {
    fn new() -> Self {
        Httop {
            stats: Arc::new(Mutex::new(Stats::new())),
            sort_by: SortBy::Count,
            display_limit: 20,
        }
    }

    fn start(&mut self) -> io::Result<()> {
        // Clone stats for log reader thread
        let stats_clone = Arc::clone(&self.stats);
        let start_time = Instant::now();

        // Thread to read logs from stdin
        thread::spawn(move || {
            let stdin = io::stdin();
            let handle = stdin.lock();

            for line in handle.lines() {
                if let Ok(line) = line {
                    if let Some(request) = parse_log_line(&line) {
                        let mut stats = stats_clone.lock().unwrap();
                        stats.update(request);

                        // Update requests per second
                        let elapsed = start_time.elapsed().as_secs_f64();
                        if elapsed > 0.0 {
                            stats.requests_per_second = stats.total_requests as f64 / elapsed;
                        }
                    }
                }
            }
        });

        // Create a channel for commands
        let (tx, rx) = mpsc::channel();

        // Spawn a thread to handle terminal input
        let tx_clone = tx.clone();
        thread::spawn(move || {
            // Try to open the terminal directly
            if let Ok(file) = File::open("/dev/tty") {
                let mut reader = io::BufReader::new(file);
                let mut buffer = String::new();

                loop {
                    buffer.clear();
                    if reader.read_line(&mut buffer).is_ok() {
                        if let Some(c) = buffer.chars().next() {
                            let is_quit = matches!(c, 'q');

                            let command = match c {
                                'q' => Command::Quit,
                                's' => Command::Sort(SortBy::StatusCode),
                                'p' => Command::Sort(SortBy::Path),
                                'c' => Command::Sort(SortBy::Count),
                                'i' => Command::Sort(SortBy::IP),
                                'u' => Command::Sort(SortBy::UserAgent),
                                '+' => Command::IncreaseLimit,
                                '-' => Command::DecreaseLimit,
                                _ => Command::Noop,
                            };

                            if tx_clone.send(command).is_err() {
                                break;
                            }

                            if is_quit {
                                break;
                            }
                        }
                    } else {
                        break;
                    }
                }
            } else {
                eprintln!("ERROR: Could not open terminal for input, controls disabled");
            }
        });

        // Main display loop
        let mut running = true;

        while running {
            // Check for commands
            if let Ok(command) = rx.try_recv() {
                match command {
                    Command::Quit => running = false,
                    Command::Sort(sort) => self.sort_by = sort,
                    Command::IncreaseLimit => self.display_limit += 5,
                    Command::DecreaseLimit => {
                        if self.display_limit > 5 {
                            self.display_limit -= 5;
                        }
                    },
                    Command::Noop => {},
                }
            }

            // Update display
            self.render_simple()?;

            thread::sleep(Duration::from_millis(500));
        }

        Ok(())
    }

    fn render_simple(&self) -> io::Result<()> {
        // Clear the terminal with simple approach
        print!("\x1B[2J\x1B[1;1H");

        // Get current stats
        let stats = self.stats.lock().unwrap().clone();

        // Display header
        let current_time = Local::now().format("%Y-%m-%d %H:%M:%S");
        println!("HTTOP (v0.1.0) - {}", current_time);
        println!("Total Requests: {} | RPS: {:.2} | Total Bytes: {}",
            stats.total_requests, stats.requests_per_second, stats.bytes_sent);
        println!();

        // Status code distribution
        println!("Status Codes:");
        let mut status_codes: Vec<_> = stats.status_codes.iter().collect();
        status_codes.sort_by(|a, b| b.1.cmp(a.1));
        for (code, count) in status_codes.iter().take(5) {
            println!("  {}: {}", code, count);
        }
        println!();

        // Display top requests heading
        println!("Top Requests (Sort: {}, Press s/p/c/i/u to change, +/- to adjust count, q to quit):",
            match self.sort_by {
                SortBy::Count => "Count",
                SortBy::Path => "Path",
                SortBy::StatusCode => "Status Code",
                SortBy::IP => "IP Address",
                SortBy::UserAgent => "User Agent",
            });

        // Table header
        println!();
        println!("+-------+-----------------+----------+---------------------------------------+------------------------------------");
        println!("| COUNT | IP              | STATUS   |  PATH                                 |  USER AGENT");
        println!("+-------+-----------------+----------+---------------------------------------+------------------------------------");

        // Gather data for display
        let mut paths_to_display: Vec<(String, usize, String, u16, String)> = Vec::new();

        for (path, count) in stats.paths.iter() {
            if let Some(req) = stats.recent_requests.iter().find(|r| &r.path == path) {
                paths_to_display.push((
                    path.clone(),
                    *count,
                    req.ip.clone(),
                    req.status_code,
                    req.user_agent.clone(),
                ));
            }
        }

        // Sort based on selected criteria
        match self.sort_by {
            SortBy::Count => paths_to_display.sort_by(|a, b| b.1.cmp(&a.1)),
            SortBy::Path => paths_to_display.sort_by(|a, b| a.0.cmp(&b.0)),
            SortBy::StatusCode => paths_to_display.sort_by(|a, b| a.3.cmp(&b.3)),
            SortBy::IP => paths_to_display.sort_by(|a, b| a.2.cmp(&b.2)),
            SortBy::UserAgent => paths_to_display.sort_by(|a, b| a.4.cmp(&b.4)),
        }

        // Display the top paths with fixed width manual formatting
        for (path, count, ip, status, user_agent) in paths_to_display.iter().take(self.display_limit) {
            let truncated_path = if path.len() > 36 {
                format!("{}...", &path[..33])
            } else {
                path.clone()
            };

            let truncated_user_agent = if user_agent.len() > 65 {
                format!("{}...", &user_agent[..64])
            } else {
                user_agent.clone()
            };

            // Manually format each field to ensure consistent spacing
            let count_str = format!(" {:<7}", count);
            let ip_str = format!("{:<16}", ip);
            let status_str = format!("{:<9}", status);
            let path_str = format!("{:<36}", truncated_path);
            let user_agent_str = format!("{:<64}", truncated_user_agent);

            println!("{}  {}  {}  {}  {}", count_str, ip_str, status_str, path_str, user_agent_str);
        }

        io::stdout().flush()?;
        Ok(())
    }
}

fn main() -> io::Result<()> {
    let mut app = Httop::new();
    app.start()
}
