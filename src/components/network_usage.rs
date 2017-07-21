use futures::{Future, Stream};
use std::sync::{Arc, Mutex};
use tokio_timer::Timer;
use tokio_core::reactor::Handle;
use component::Component;
use error::{Error, Result};
use std::time::{Instant, Duration};

#[derive(Clone, PartialEq, Copy)]
pub enum Scale {
    Binary,
    Decimal,
}

impl Scale {
    fn base(&self) -> u16 {
        match *self {
            Scale::Decimal => 1000,
            Scale::Binary => 1024,
        }
    }
}

#[derive(Clone, Copy)]
pub enum Direction {
    Incoming,
    Outgoing,
}

pub struct NetworkUsage {
    pub interface: String,
    pub direction: Direction,
    pub scale: Scale,
    pub percision: u8,
    pub refresh_frequency: Duration,
    pub sample_duration: Duration,
    pub buffer: Vec<u64>,
    pub buffer_size: usize,
}

impl Default for NetworkUsage {
    fn default() -> NetworkUsage {
        let buffer_size = 5;
        NetworkUsage {
            interface: "eth0".to_string(),
            direction: Direction::Incoming,
            scale: Scale::Binary,
            percision: 3,
            refresh_frequency: Duration::from_secs(10),
            sample_duration: Duration::from_secs(1),
            buffer: Vec::with_capacity(buffer_size),
            buffer_size,
        }
    }
}

fn get_prefix(scale: Scale, power: u8) -> &'static str {
    match (scale, power) {
        (Scale::Decimal, 0) | (Scale::Binary, 0) => "B/s",
        (Scale::Decimal, 1) => "kb/s",
        (Scale::Decimal, 2) => "Mb/s",
        (Scale::Decimal, 3) => "Gb/s",
        (Scale::Decimal, 4) => "Tb/s",
        (Scale::Binary, 1) => "KiB/s",
        (Scale::Binary, 2) => "MiB/s",
        (Scale::Binary, 3) => "GiB/s",
        (Scale::Binary, 4) => "TiB/s",
        _ => "X/s",
    }
}

fn get_number_scale(number: u64, scale: Scale) -> (f64, u8) {
    let log = (number as f64).log(scale.base() as f64);
    let wholes = log.floor();
    let over = (scale.base() as f64).powf(log - wholes);
    (over, wholes as u8)
}

fn get_bytes(interface: &str, dir: Direction) -> ::std::io::Result<Option<u64>> {
    let dev = ::procinfo::net::dev::dev()?
        .into_iter()
        .find(|dev| dev.interface == interface);

    let dev = match dev {
        Some(dev) => dev,
        None => return Ok(None),
    };

    match dir {
        Direction::Incoming => Ok(Some(dev.receive_bytes)),
        Direction::Outgoing => Ok(Some(dev.transmit_bytes)),
    }
}

impl Component for NetworkUsage {
    type Error = Error;
    type Stream = Box<Stream<Item = String, Error = Error>>;

    fn init(&mut self) -> Result<()> {
        ::procinfo::net::dev::dev()?
            .into_iter()
            .find(|dev| dev.interface == self.interface)
            .ok_or_else(|| Error::from("No such network interface"))?;
        Ok(())
    }

    // type Stream = Box<Stream<Item = String, Error = Error>>;
    fn stream(self, _: Handle) -> Self::Stream {
        let timer = Timer::default();
        let conf = Arc::new(Mutex::new(self));

        timer
            .interval_at(Instant::now(), Duration::from_secs(1))
            .and_then(move |()| {
                let conf = conf.clone();
                let conf2 = conf.clone();
                let first = {
                    let lock = conf.lock().unwrap();
                    get_bytes(&lock.interface, lock.direction).unwrap().unwrap()
                };
                timer
                    .sleep(Duration::from_secs(1))
                    .and_then(move |()| {
                        // Get throughput by getting second reading and checking diff
                        let mut lock = conf.lock().unwrap();
                        let second = get_bytes(&lock.interface, lock.direction).unwrap().unwrap();
                        let per_second = (second - first) / lock.sample_duration.as_secs();

                        // Add current throughput to buffer
                        lock.buffer.insert(0, per_second);
                        while lock.buffer.len() > lock.buffer_size {
                            let _ = lock.buffer.pop();
                        }

                        // Get average from buffer
                        Ok(lock.buffer.iter().sum::<u64>() / lock.buffer.len() as u64)
                    })
                    .map(move |speed| {
                        let lock = conf2.lock().unwrap();
                        let (num, power) = get_number_scale(speed, lock.scale);
                        let x = 10f64.powi((lock.percision-1) as i32);
                        let num = (num*x).round() / x;
                        format!("{} {}", num, get_prefix(lock.scale, power))
                    })
            })
            .map_err(|_| "timer error".into())
            .boxed()
    }
}
