use std::{
    io::{stdin, Write},
    thread::{self, sleep},
    time::Duration,
};

fn main() {
    let (tx, rx) = circular_channel::circular_channel::<i16>(100);

    let _sender = thread::spawn(move || {
        let mut counter = 0i16;
        loop {
            tx.send(counter);
            counter = counter.wrapping_add(1);
            sleep(Duration::from_nanos(10));
        }
    });

    let _receiver = thread::spawn(move || -> ! {
        let stdout = std::io::stdout();
        let mut stdout = stdout.lock();
        let mut prev: Option<i16> = None;
        loop {
            if let Some(val) = rx.recv() {
                match prev {
                    None => writeln!(stdout, "Received: {val}").unwrap(),
                    Some(prev) if val.wrapping_sub(prev) == 1 => {
                        writeln!(stdout, "Received: {val} fast enough").unwrap()
                    }
                    Some(prev) => writeln!(
                        stdout,
                        "Received: {val}, skipped: {}",
                        val.wrapping_sub(prev)
                    )
                    .unwrap(),
                }
                prev = Some(val);
            }
            thread::sleep(Duration::from_nanos(5));
        }
    });

    let stdin = stdin();
    let mut line = String::new();
    let _ = stdin.read_line(&mut line);
}
