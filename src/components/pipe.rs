use component::Component;
use tokio_core::reactor::Handle;
use std::io::BufReader;
use std::process::{Stdio, Command};
use tokio_process::{ChildStdout, CommandExt};
use futures::{Stream, Future};
use tokio_timer::Timer;
use std::time::{Duration, Instant};
use error::Error;

pub struct Pipe {
    pub command: String,
    pub args: Vec<String>,
    pub refresh_rate: Option<Duration>,
}

impl Default for Pipe {
    fn default() -> Pipe {
        Pipe {
            command: "true".to_string(),
            args: vec![],
            refresh_rate: None,
        }
    }
}

impl Pipe {
    fn reader(&self, handle: &Handle) -> BufReader<ChildStdout> {
        let mut cmd = Command::new(self.command.as_str());
        cmd.args(self.args.as_slice());
        cmd.stdin(Stdio::inherit()).stdout(Stdio::piped());
        let mut child = cmd.spawn_async(&handle).unwrap();
        let stdout = child.stdout().take().unwrap();
        handle.spawn(child.map(|_| ()).map_err(|_| ()));
        BufReader::new(stdout)
    }
}

impl Component for Pipe {
    type Error = Error;
    type Stream = Box<Stream<Item = String, Error = Error>>;

    fn stream(self, handle: Handle) -> Self::Stream {
        if let Some(refresh_rate) = self.refresh_rate {
            let timer = Timer::default();
            Box::new(
                timer
                    .interval_at(Instant::now(), refresh_rate)
                    .and_then(move |_| Ok(::tokio_io::io::lines(self.reader(&handle))))
                    .flatten()
                    .map_err(Error::from),
            )
        } else {
            Box::new(::tokio_io::io::lines(self.reader(&handle)).from_err())
        }
    }
}
