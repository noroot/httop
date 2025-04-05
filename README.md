# HTTOP

A real-time HTTP log monitor similar to ApacheTop, written in Rust. This tool allows you to watch incoming http requests in real-time. Purpose - research on web crawlers.

## Features

- Processes http log data in real-time through pipe from `tail -f`
- Displays overall statistics (requests per second, total bytes, status code distribution)

## Roadmap

- Filtering by Crawler bots to research on crawlers
- Indicate "bad" crawlers

## Installation

### Prerequisites

- Rust and Cargo (install from [rust-lang.org](https://www.rust-lang.org/tools/install))
- Nginx server with access logs

### Building from Source

1. Clone the repository:
   ```
   git clone https://github.com/yourusername/nginx-top.git
   cd nginx-top
   ```

2. Build the project:
   ```
   cargo build --release
   ```

3. The compiled binary will be available at `target/release/nginx-top`

## Usage

Pipe Nginx log data to NginxTop:

```
tail -f /var/log/nginx/access.log | httop
```

### Interactive Controls

Type the following characters and press Enter to control the display:

- `s`: Sort by Status Code
- `p`: Sort by Path
- `c`: Sort by Count (default)
- `i`: Sort by IP Address
- `u`: Sort by User Agent
- `+`: Increase number of displayed entries
- `-`: Decrease number of displayed entries
- `q`: Quit

## Nginx Log Format Compatibility

NginxTop is configured to parse the standard Nginx log format:

```
log_format main '$remote_addr - $remote_user [$time_local] "$request" '
                '$status $body_bytes_sent "$http_referer" '
                '"$http_user_agent" $request_time';
```

If your Nginx uses a different log format, you may need to modify the regex pattern in the `parse_log_line` function.

## Sample Output

```
Total Requests: 1548 | RPS: 32.50 | Total Bytes: 28945213

Status Codes:
  200: 1423
  404: 87
  302: 34
  500: 4

Top Requests (Sort: Count, Press s/p/c/i/u to change, +/- to adjust count, q to quit):

COUNT    IP              STATUS     PATH                                    USER AGENT
-------  --------------  ---------  --------------------------------------  -----------------------------------
183      192.168.1.45    200        /index.html                             Mozilla/5.0 (Windows NT 10.0; Win...
127      192.168.1.22    200        /assets/css/main.css                    Mozilla/5.0 (Macintosh; Intel Mac...
98       192.168.1.60    200        /assets/js/app.js                       Mozilla/5.0 (iPhone; CPU iPhone O...
76       192.168.1.33    200        /api/users                              PostmanRuntime/7.29.0
```

## License

GNU v3

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the project
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request
