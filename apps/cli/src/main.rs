//! rustfig-cli: convert, inspect and benchmark figures; run the MCP server
//! (the command line is in the library, which `rustfig` shares).

fn main() -> std::process::ExitCode {
    rustfig_cli::run()
}
