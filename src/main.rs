use std::env;
use log::{info, warn, error, debug};
use anyhow::Result;
use clap::Parser;
use analytics::analytics;

#[derive(Parser)]
struct Cli {
    #[arg(short, long)]
    production: bool,
}

fn main() -> Result<()> { -> Result<()> {
    let cli = Cli::parse();
    let config = analytics::Config { 
        production_mode: cli.production, 
        ..Default::default() 
    };
    let mut component = analytics::new(config);
    
    component.initialize()?;
    component.start()?;
    
    println!("SigmaOS-analytics running");
    Ok(())
}

#[derive(Debug, Clone)]
struct Config {
    verbose: bool,
    dry_run: bool,
}

impl Config {
    fn from_env() -> Self {
        Config {
            verbose: env::var("VERBOSE").is_ok(),
            dry_run: env::var("DRY_RUN").is_ok(),
        }
    }
}
