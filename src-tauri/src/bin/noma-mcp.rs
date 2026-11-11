//! `noma-mcp`: NOMA's local MCP server over stdio. Point an MCP client (Claude
//! Desktop, an IDE…) at this binary; see Settings → Your data in NOMA.

fn main() {
    std::process::exit(noma_lib::data::mcp::run());
}
