use std::sync::Arc;

use rmcp::handler::server::tool::{
    Callee, FromToolCallContextPart, ToolBox, ToolBoxItem, ToolCallContext,
    cached_schema_for_type, Parameters,
};
use rmcp::model::{CallToolResult, Content, Tool};
use rmcp::Error as McpError;

use mdtool_core::domain::policy::FilesystemPolicy;
use mdtool_core::services::block_service::BlockService;
use mdtool_core::services::ascii_service::AsciiService;

use crate::schema::*;

/// The MCP server handler. Holds service instances behind Arc so the handler
/// is Clone (required by rmcp's ServerHandler).
#[derive(Clone)]
pub struct MdtoolServer {
    block_service: Arc<BlockService>,
    ascii_service: Arc<AsciiService>,
}

impl MdtoolServer {
    pub fn new() -> Self {
        let policy = FilesystemPolicy::default();
        let block_service = Arc::new(BlockService::new(policy.clone()));
        let ascii_service = Arc::new(AsciiService::new(policy));
        Self {
            block_service,
            ascii_service,
        }
    }

    /// Build the static ToolBox with all 7 registered tools.
    pub fn tool_box() -> &'static ToolBox<Self> {
        static TOOL_BOX: std::sync::OnceLock<ToolBox<MdtoolServer>> = std::sync::OnceLock::new();
        TOOL_BOX.get_or_init(|| {
            let mut tb = ToolBox::new();

            tb.add(make_tool::<ReadOutlineRequest, _>(
                "markdown_read_outline",
                "Return heading outline of a markdown document as a flat list of sections with id, level, title, and optionally canonical path. Use this first to understand document structure before targeting specific blocks. Set include_paths=true only when you need path-based selectors.",
                cached_schema_for_type::<ReadOutlineRequest>(),
                |srv, params| {
                    let args = params.0;
                    match srv.block_service.read_outline(&args.file_path, args.max_depth, args.include_paths) {
                        Ok(resp) => ok_json(resp),
                        Err(e) => err_text(&e),
                    }
                },
            ));

            tb.add(make_tool::<ReadBlockRequest, _>(
                "markdown_read_block",
                "Read block data from a markdown document. The view parameter controls what is returned: data (full block details), text (raw text content only), tree (block subtree as nested structure), children (ordered child blocks), or type (all blocks of a given type). Defaults to data.",
                cached_schema_for_type::<ReadBlockRequest>(),
                |srv, params| {
                    let args = params.0;
                    let selector = args.selector.unwrap_or_default();
                    let include_text = args.include_text.unwrap_or(true);

                    match args.view.as_str() {
                        "text" => {
                            match srv.block_service.read_block_text(&args.file_path, &selector) {
                                Ok(resp) => ok_json(resp),
                                Err(e) => err_text(&e),
                            }
                        }
                        "tree" => {
                            let depth = args.depth.unwrap_or(-1);
                            match srv.block_service.read_block_tree(
                                &args.file_path,
                                &selector,
                                depth,
                                include_text,
                            ) {
                                Ok(resp) => ok_json(resp),
                                Err(e) => err_text(&e),
                            }
                        }
                        "children" => {
                            match srv.block_service.read_block_children(&args.file_path, &selector) {
                                Ok(resp) => ok_json(resp),
                                Err(e) => err_text(&e),
                            }
                        }
                        "by_type" => {
                            let bt = args.block_type.as_deref().unwrap_or("fence");
                            match srv.block_service.read_blocks_by_type(&args.file_path, bt) {
                                Ok(resp) => ok_json(resp),
                                Err(e) => err_text(&e),
                            }
                        }
                        _ => {
                            // default "block" view
                            match srv
                                .block_service
                                .read_block(&args.file_path, &selector, include_text)
                            {
                                Ok(resp) => ok_json(resp),
                                Err(e) => err_text(&e),
                            }
                        }
                    }
                },
            ));

            tb.add(make_tool::<SearchRequest, _>(
                "markdown_search",
                "Search for text across blocks in a markdown document. Returns matching block IDs and matched line text. Optionally scope to a subtree via selector. Case-insensitive by default.",
                cached_schema_for_type::<SearchRequest>(),
                |srv, params| {
                    let args = params.0;
                    match srv.block_service.search_blocks(
                        &args.file_path,
                        &args.query,
                        args.selector.as_ref(),
                        args.case_sensitive,
                    ) {
                        Ok(resp) => ok_json(resp),
                        Err(e) => err_text(&e),
                    }
                },
            ));

            tb.add(make_tool::<EditRequest, _>(
                "markdown_edit",
                "Edit one or more blocks in a markdown document. Pass a single operation for single edits, or an array for batch (single patch-reparse cycle). Defaults to dry_run=true.",
                cached_schema_for_type::<EditRequest>(),
                |srv, params| {
                    let args = params.0;
                    match srv
                        .block_service
                        .edit(&args.file_path, &args.operations, args.dry_run)
                    {
                        Ok(resp) => ok_json(resp),
                        Err(e) => err_text(&e),
                    }
                },
            ));

            tb.add(make_tool::<ValidateRequest, _>(
                "markdown_validate",
                "Validate a markdown document for structural issues. Returns diagnostics with severity, code, message, and optional suggested fix.",
                cached_schema_for_type::<ValidateRequest>(),
                |srv, params| {
                    let args = params.0;
                    match srv.block_service.validate(&args.file_path) {
                        Ok(resp) => ok_json(resp),
                        Err(e) => err_text(&e),
                    }
                },
            ));

            tb.add(make_tool::<NormalizeRequest, _>(
                "markdown_normalize",
                "Normalize formatting of a markdown document without changing semantic content. Preserves fenced code contents by default. Defaults to dry_run=true.",
                cached_schema_for_type::<NormalizeRequest>(),
                |srv, params| {
                    let args = params.0;
                    match srv
                        .block_service
                        .normalize(&args.file_path, args.options, args.dry_run)
                    {
                        Ok(resp) => ok_json(resp),
                        Err(e) => err_text(&e),
                    }
                },
            ));

            tb.add(make_tool::<FormatAsciiRequest, _>(
                "markdown_format_ascii",
                "Format or repair ASCII art diagrams inside fenced code blocks (info strings: ascii, box, diagram). Modes: format_only (safe), repair_safe (conservative border alignment). Defaults to dry_run=true.",
                cached_schema_for_type::<FormatAsciiRequest>(),
                |srv, params| {
                    let args = params.0;
                    match srv
                        .ascii_service
                        .format_ascii(&args.file_path, args.mode, args.dry_run)
                    {
                        Ok(resp) => ok_json(resp),
                        Err(e) => err_text(&e),
                    }
                },
            ));

            tb
        })
    }
}

// ---------------------------------------------------------------------------
// Tool registration helper
// ---------------------------------------------------------------------------

/// Create a ToolBoxItem from a name, description, JSON schema, and a handler
/// closure. The handler receives `(&MdtoolServer, Parameters<T>)` and returns
/// a synchronous `Result<CallToolResult, McpError>`.
fn make_tool<T, F>(
    name: &'static str,
    description: &'static str,
    schema: Arc<rmcp::model::JsonObject>,
    handler: F,
) -> ToolBoxItem<MdtoolServer>
where
    T: serde::de::DeserializeOwned + Send + 'static,
    F: Fn(&MdtoolServer, Parameters<T>) -> Result<CallToolResult, McpError>
        + Send
        + Sync
        + 'static,
{
    let tool = Tool::new(name, description, schema);
    ToolBoxItem::new(tool, move |ctx: ToolCallContext<'_, MdtoolServer>| {
        // Extract Callee (the &MdtoolServer reference) and Parameters from the
        // context using rmcp's FromToolCallContextPart trait.
        let (Callee(srv), ctx) = match Callee::from_tool_call_context_part(ctx) {
            Ok(pair) => pair,
            Err(e) => return Box::pin(std::future::ready(Err(e))),
        };
        let (params, _ctx) = match Parameters::<T>::from_tool_call_context_part(ctx) {
            Ok(pair) => pair,
            Err(e) => return Box::pin(std::future::ready(Err(e))),
        };
        let result = handler(srv, params);
        Box::pin(std::future::ready(result))
    })
}

// ---------------------------------------------------------------------------
// Utility functions
// ---------------------------------------------------------------------------

fn ok_json(val: impl serde::Serialize) -> Result<CallToolResult, McpError> {
    let json = serde_json::to_string(&val).unwrap_or_else(|e| format!("{{\"error\": \"{e}\"}}"));
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

fn err_text(e: &dyn std::fmt::Display) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::error(vec![Content::text(e.to_string())]))
}
