//! Heimsight CLI
//!
//! Command-line interface for interacting with the Heimsight observability platform.
//!
//! # Usage
//!
//! ```bash
//! heimsight --help
//! heimsight health
//! heimsight logs --service api --level error
//! ```

#![deny(unsafe_code)]

use clap::{Parser, Subcommand};

/// Heimsight CLI - Observability platform command-line interface
#[derive(Parser)]
#[command(name = "heimsight")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// API server URL
    #[arg(
        short,
        long,
        env = "HEIMSIGHT_API_URL",
        default_value = "http://localhost:8080"
    )]
    api_url: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Check API server health
    Health,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Health) => {
            println!("Checking health of Heimsight API at {}...", cli.api_url);
            println!("Health check not yet implemented");
        }
        None => {
            println!("Heimsight CLI v{}", env!("CARGO_PKG_VERSION"));
            println!("Use --help for usage information");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse() {
        // Verify CLI can parse without arguments
        let cli = Cli::try_parse_from(["heimsight"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_health_command() {
        let cli = Cli::try_parse_from(["heimsight", "health"]);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        assert!(matches!(cli.command, Some(Commands::Health)));
    }
}
