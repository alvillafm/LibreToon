//! Bounded capture for native helpers that decode untrusted media.
use std::{
    io::{self, Read},
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

fn capture(mut pipe: impl Read, limit: usize, exceeded: &AtomicBool) -> io::Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut buffer = [0; 8192];
    loop {
        let count = pipe.read(&mut buffer)?;
        if count == 0 {
            return Ok(output);
        }
        if count > limit - output.len() {
            exceeded.store(true, Ordering::Relaxed);
            return Err(io::Error::other("Helper output exceeded its limit"));
        }
        let required = output.len() + count;
        if required > output.capacity() {
            let capacity = required.max(output.capacity().saturating_mul(2)).min(limit);
            output
                .try_reserve_exact(capacity - output.len())
                .map_err(io::Error::other)?;
        }
        output.extend_from_slice(&buffer[..count]);
    }
}

pub(crate) fn output(
    command: &mut Command,
    timeout: Duration,
    stdout_limit: usize,
    stderr_limit: usize,
) -> io::Result<Output> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout = child.stdout.take().expect("configured stdout pipe");
    let stderr = child.stderr.take().expect("configured stderr pipe");
    let exceeded = AtomicBool::new(false);
    let began = Instant::now();
    std::thread::scope(|scope| {
        let out = scope.spawn(|| capture(stdout, stdout_limit, &exceeded));
        let err = scope.spawn(|| capture(stderr, stderr_limit, &exceeded));
        let status = loop {
            if exceeded.load(Ordering::Relaxed) {
                break Err(io::Error::other("Helper output exceeded its limit"));
            }
            match child.try_wait() {
                Ok(Some(status)) => break Ok(status),
                Ok(None) if began.elapsed() < timeout => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Ok(None) => break Err(io::Error::new(io::ErrorKind::TimedOut, "Helper timed out")),
                Err(error) => break Err(error),
            }
        };
        if status.is_err() {
            let _ = child.kill();
            let _ = child.wait();
        }
        let stdout = out
            .join()
            .map_err(|_| io::Error::other("Helper output reader stopped"))?;
        let stderr = err
            .join()
            .map_err(|_| io::Error::other("Helper error reader stopped"))?;
        Ok(Output {
            status: status?,
            stdout: stdout?,
            stderr: stderr?,
        })
    })
}

pub(crate) trait BoundedOutput {
    fn bounded_output(&mut self, timeout_seconds: u64, stdout_limit: usize) -> io::Result<Output>;
}

impl BoundedOutput for Command {
    fn bounded_output(&mut self, timeout_seconds: u64, stdout_limit: usize) -> io::Result<Output> {
        output(
            self,
            Duration::from_secs(timeout_seconds),
            stdout_limit,
            1024 * 1024,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn overdue_helper_is_stopped_without_waiting_for_its_script() {
        let executable = crate::ffmpeg_setup::system_program("WindowsPowerShell/v1.0/powershell");
        let mut command = Command::new(executable);
        crate::exporter::hidden(&mut command);
        command.args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Start-Sleep -Seconds 10",
        ]);
        let start = Instant::now();
        let result = output(&mut command, Duration::from_millis(75), 1024, 1024);
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::TimedOut);
        assert!(start.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn output_capture_rejects_excess_without_growing_the_buffer() {
        let exceeded = AtomicBool::new(false);
        assert_eq!(capture(&b"ok"[..], 2, &exceeded).unwrap(), b"ok");
        assert!(!exceeded.load(Ordering::Relaxed));
        assert!(capture(&b"too much"[..], 2, &exceeded).is_err());
        assert!(exceeded.load(Ordering::Relaxed));
    }
}
