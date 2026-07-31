import { handleReadDocumentRequest } from "./read-document";
import { handleReadStyleRequest } from "./read-styles";
import { handleReadExportRequest } from "./read-export";

type RequestHandler = (request: any) => Promise<any | null>;

// Registry maps request type → handler for O(1) dispatch
const handlers: Map<string, RequestHandler> = new Map();

const register = (handler: RequestHandler, types: string[]) => {
  for (const t of types) handlers.set(t, handler);
};

// Probe each handler to discover its supported types at registration time
const discoverTypes = async (handler: RequestHandler): Promise<string[]> => {
  const types: string[] = [];
  // Each handler is a switch — we can't introspect it, so we pass a sentinel
  // and collect the type from the handler's internal match.
  // Instead, we use explicit registration below.
  return types;
};

// Explicit registration — single source of truth for handler routing
register(handleReadDocumentRequest, [
  "get_selection",
  "get_nodes_info",
  "get_design_context",
  "get_metadata",
  "get_pages",
  "get_viewport",
  "get_fonts",
  "search_nodes",
  "get_reactions",
  "scan_nodes_by_types",
  "get_plugin_data",
]);
register(handleReadStyleRequest, [
  "get_styles",
  "get_variable_defs",
  "get_local_components",
  "get_annotations",
  "export_tokens",
]);
register(handleReadExportRequest, [
  "get_screenshot",
  "export_frames_to_pdf",
  "export_nodes",
]);

export const handleReadRequest = async (request: any) => {
  const handler = handlers.get(request.type);
  if (handler) return await handler(request);
  return null;
};

// Export for other modules to check if a type is registered
export const isReadType = (type: string): boolean => handlers.has(type);
