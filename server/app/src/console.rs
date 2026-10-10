//! Bounded local input for the two supported administrative commands.

use std::io::{BufRead, BufReader};

use tokio::sync::mpsc;

const MAX_LINE: usize = 256;

/// Dedicated input thread avoids Tokio stdin's uncancellable runtime shutdown.
/// Sixteen bounded lines absorb terminal bursts; backpressure stops the reader.
pub(super) fn input() -> mpsc::Receiver<Result<String, String>> {
    let (sender, receiver) = mpsc::channel(16);
    std::thread::spawn(move || {
        let reader = BufReader::new(std::io::stdin());
        read_input(reader, |line| sender.blocking_send(line).is_ok());
    });
    receiver
}

fn read_input(reader: impl BufRead, mut deliver: impl FnMut(Result<String, String>) -> bool) {
    let mut line = Vec::with_capacity(MAX_LINE);
    let mut oversized = false;
    for byte in reader.bytes() {
        let byte = match byte {
            Ok(byte) => byte,
            Err(error) => {
                deliver(Err(format!("console read failed: {error}")));
                return;
            }
        };
        if byte == b'\n' {
            let input = if oversized {
                Err("console command exceeds 256 bytes".to_owned())
            } else {
                String::from_utf8(std::mem::take(&mut line))
                    .map_err(|_| "console command is not UTF-8".to_owned())
            };
            if !deliver(input) {
                return;
            }
            line.clear();
            oversized = false;
        } else if line.len() < MAX_LINE {
            line.push(byte);
        } else {
            oversized = true;
        }
    }
    if !line.is_empty() || oversized {
        deliver(if oversized {
            Err("console command exceeds 256 bytes".to_owned())
        } else {
            String::from_utf8(line).map_err(|_| "console command is not UTF-8".to_owned())
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_console_recovers_after_oversized_and_invalid_lines() {
        let mut bytes = vec![b'x'; MAX_LINE + 1];
        bytes.extend_from_slice(b"\n\xff\nop Admin\ndeop Admin");
        let mut lines = Vec::new();
        read_input(bytes.as_slice(), |line| {
            lines.push(line);
            true
        });
        assert!(lines[0].is_err());
        assert!(lines[1].is_err());
        assert_eq!(lines[2], Ok("op Admin".to_owned()));
        assert_eq!(lines[3], Ok("deop Admin".to_owned()));
    }
}
