use sigmaos_analytics::{Analytics, Event};
use std::path::Path;

fn usage() -> ! {
    eprintln!("usage:");
    eprintln!("  sigmaos-analytics record FILE CATEGORY ACTION [VALUE] [TAG]*");
    eprintln!("  sigmaos-analytics counter FILE CATEGORY ACTION");
    eprintln!("  sigmaos-analytics sum FILE CATEGORY ACTION");
    eprintln!("  sigmaos-analytics histogram FILE CATEGORY ACTION BINS");
    eprintln!("  sigmaos-analytics count FILE CATEGORY");
    eprintln!("  sigmaos-analytics info FILE");
    std::process::exit(2);
}

fn load(file: &str) -> Analytics {
    if !Path::new(file).exists() {
        let a = Analytics::new();
        a.save(file).unwrap();
        a
    } else {
        Analytics::load(file).unwrap()
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        usage();
    }
    let cmd = args[0].as_str();
    let file = args[1].as_str();

    match cmd {
        "record" => {
            if args.len() < 4 {
                usage();
            }
            let category = args[2].as_str();
            let action = args[3].as_str();
            let value = args.get(4).and_then(|s| s.parse::<f64>().ok());
            let tags: Vec<String> = args.get(5).map(|_| args[5..].to_vec()).unwrap_or_default();
            let mut e = Event::new(category, action);
            if let Some(v) = value {
                e = e.with_value(v);
            }
            for tag in tags {
                e = e.with_tag(tag);
            }
            let mut a = load(file);
            a.record(e);
            a.save(file).unwrap();
            println!("recorded event");
        }
        "counter" => {
            if args.len() < 4 {
                usage();
            }
            let a = load(file);
            println!("{}", a.get_counter(args[2].as_str(), args[3].as_str()));
        }
        "sum" => {
            if args.len() < 4 {
                usage();
            }
            let a = load(file);
            println!("{}", a.get_sum(args[2].as_str(), args[3].as_str()));
        }
        "histogram" => {
            if args.len() < 5 {
                usage();
            }
            let a = load(file);
            let bins: usize = args[4].parse().unwrap_or(10);
            let hist = a.histogram(args[2].as_str(), args[3].as_str(), bins);
            for (lo, hi, c) in hist {
                println!("{}-{}: {}", lo, hi, c);
            }
        }
        "count" => {
            if args.len() < 3 {
                usage();
            }
            let a = load(file);
            println!("{}", a.count_by_category(args[2].as_str()));
        }
        "info" => {
            let a = load(file);
            println!("{} events", a.event_count());
        }
        _ => usage(),
    }
}
