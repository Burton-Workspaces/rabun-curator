mod cache;
mod graph;
mod init;
mod links;
mod report;
mod search;
mod server;
mod setup;
mod tools;
mod vault;
mod version;
mod viz;
mod wiki;

use clap::{Parser, Subcommand};
use rmcp::ServiceExt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use cache::VaultCache;
use server::CuratorServer;

/// Karpathy LLM Wiki — local MCP server for a compounding markdown knowledge base.
#[derive(Parser)]
#[command(name = env!("CARGO_PKG_NAME"), version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Vault paths to serve (can specify multiple)
    #[arg(value_name = "VAULT_PATH")]
    vaults: Vec<PathBuf>,

    /// Configure Claude, Grok, and Cursor MCP + skills
    #[arg(long)]
    setup: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Create raw/, wiki/, index.md, log.md, and AGENTS.md
    Init {
        /// Directory to initialize (created if missing)
        path: PathBuf,
    },
}

fn resolve_vaults(cli: &Cli) -> Vec<PathBuf> {
    if !cli.vaults.is_empty() {
        return cli.vaults.clone();
    }
    if let Ok(vaults) = std::env::var("RABUN_CURATOR_VAULTS") {
        return vaults.split(':').map(PathBuf::from).collect();
    }
    if let Ok(vault) = std::env::var("RABUN_CURATOR_VAULT") {
        return vec![PathBuf::from(vault)];
    }
    Vec::new()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if let Some(Commands::Init { path }) = &cli.command {
        init::run(path)?;
        return Ok(());
    }

    let library_paths = resolve_vaults(&cli);

    if cli.setup {
        if library_paths.is_empty() {
            eprintln!("Error: --setup requires at least one vault path.");
            eprintln!("Usage: rabun-curator --setup /path/to/vault");
            std::process::exit(1);
        }
        setup::run_setup(&library_paths)?;
        return Ok(());
    }

    if library_paths.is_empty() {
        eprintln!("Error: no vault specified.");
        eprintln!("Usage: rabun-curator /path/to/vault");
        eprintln!("       rabun-curator --setup /path/to/vault");
        eprintln!("       rabun-curator init /path/to/vault");
        std::process::exit(1);
    }

    for path in &library_paths {
        if !path.exists() {
            eprintln!("Warning: vault path does not exist: {}", path.display());
        }
    }

    let link_stoplist = CuratorServer::build_link_stoplist(&library_paths);
    let server = CuratorServer {
        roots: library_paths.clone(),
        link_stoplist,
        cache: Arc::new(Mutex::new(VaultCache::default())),
        tool_router: CuratorServer::new_tool_router(),
    };
    let vault_cache = VaultCache::build_full(&server);
    *server.cache.lock().unwrap() = vault_cache;

    let display: Vec<_> = library_paths
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    let version = version::crate_version();
    if display.len() == 1 {
        eprintln!(
            "rabun-curator {version} MCP starting — vault: {}",
            display[0]
        );
    } else {
        eprintln!(
            "rabun-curator {version} MCP starting — {} vaults: {}",
            display.len(),
            display.join(", ")
        );
    }

    let transport = rmcp::transport::stdio();
    let service = server.serve(transport).await?;
    service.waiting().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::Cli;

    #[test]
    fn clap_debug_assert() {
        Cli::command().debug_assert();
    }

    #[test]
    fn crate_version_is_semver_2() {
        let parsed = crate::version::crate_version();
        assert!(parsed.pre.is_empty());
        assert_eq!(
            parsed,
            crate::version::parse(env!("CARGO_PKG_VERSION")).unwrap()
        );
        assert_eq!(
            crate::version::crate_git_tag(),
            format!("v{}", env!("CARGO_PKG_VERSION"))
        );
    }
}
