use anyhow::Result;
use clap::Parser;
use analytics::analytics;

#[derive(Parser)]
struct Cli {
    #[arg(short, long)]
    production: bool,
}

fn main() -> Result<()> {
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
