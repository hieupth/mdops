mod schema;
mod tools;

use rmcp::handler::server::ServerHandler;
use rmcp::model::{
    CallToolRequestParam, CallToolResult, Implementation, ListToolsResult,
    PaginatedRequestParam, ServerCapabilities, ServerInfo,
};
use rmcp::service::{RequestContext, RoleServer, ServiceExt};
use rmcp::transport::stdio;
use tools::MdtoolServer;

/// Implement the rmcp `ServerHandler` trait by delegating `list_tools` and
/// `call_tool` to the static `ToolBox`, and providing server metadata for
/// the MCP initialize handshake.
impl ServerHandler for MdtoolServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: rmcp::model::ProtocolVersion::default(),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation {
                name: "mdtool-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some("mdtool MCP server — tools for reading, editing, searching, validating, normalizing, and formatting Markdown documents.".to_string()),
        }
    }

    async fn list_tools(
        &self,
        _request: PaginatedRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, rmcp::Error> {
        Ok(ListToolsResult {
            next_cursor: None,
            tools: MdtoolServer::tool_box().list(),
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, rmcp::Error> {
        let ctx = rmcp::handler::server::tool::ToolCallContext::new(self, request, context);
        MdtoolServer::tool_box().call(ctx).await
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing (writes to stderr so it does not interfere with
    // the JSON-RPC transport on stdout).
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "mdtool_mcp=info".parse().unwrap()),
        )
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("starting mdtool MCP server on stdio");

    let server = MdtoolServer::new();
    let transport = stdio();

    // `serve_server` handles the full initialize handshake then enters the
    // message loop. It returns when the client closes the connection.
    let running = server.serve(transport).await?;
    running.waiting().await?;

    Ok(())
}
