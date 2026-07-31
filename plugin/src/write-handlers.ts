import { handleWriteCreateRequest } from "./write-create";
import { handleWriteModifyRequest } from "./write-modify";
import { handleWriteStyleRequest } from "./write-styles";
import { handleWriteVariableRequest } from "./write-variables";
import { handleWriteComponentRequest } from "./write-components";
import { handleWritePrototypeRequest } from "./write-prototype";
import { handleWritePageRequest } from "./write-page";

type RequestHandler = (request: any) => Promise<any | null>;

// Registry maps request type → handler for O(1) dispatch
const handlers: Map<string, RequestHandler> = new Map();

const register = (handler: RequestHandler, types: string[]) => {
  for (const t of types) handlers.set(t, handler);
};

register(handleWriteCreateRequest, [
  "create_frame",
  "create_rectangle",
  "create_ellipse",
  "create_text",
  "import_image",
  "create_component",
  "create_section",
  "create_line",
  "create_star",
  "create_polygon",
  "batch_create_nodes",
]);
register(handleWriteModifyRequest, [
  "set_text",
  "set_text_properties",
  "set_fills",
  "set_strokes",
  "move_nodes",
  "resize_nodes",
  "rename_node",
  "clone_node",
  "set_opacity",
  "set_corner_radius",
  "set_auto_layout",
  "set_visible",
  "set_locked",
  "rotate_nodes",
  "reorder_nodes",
  "set_blend_mode",
  "set_constraints",
  "reparent_nodes",
  "batch_rename_nodes",
  "find_replace_text",
  "set_gradient_fill",
  "set_viewport",
  "set_plugin_data",
  "set_text_range",
]);
register(handleWriteStyleRequest, [
  "create_paint_style",
  "create_text_style",
  "create_effect_style",
  "create_grid_style",
  "update_paint_style",
  "update_text_style",
  "update_effect_style",
  "update_grid_style",
  "delete_style",
  "apply_style_to_node",
  "set_effects",
  "bind_variable_to_node",
]);
register(handleWriteVariableRequest, [
  "create_variable_collection",
  "add_variable_mode",
  "create_variable",
  "set_variable_value",
  "delete_variable",
]);
register(handleWriteComponentRequest, [
  "swap_component",
  "detach_instance",
  "delete_nodes",
  "navigate_to_page",
  "group_nodes",
  "ungroup_nodes",
  "set_component_property",
]);
register(handleWritePrototypeRequest, [
  "set_reactions",
  "remove_reactions",
]);
register(handleWritePageRequest, [
  "add_page",
  "delete_page",
  "rename_page",
]);

export const handleWriteRequest = async (request: any) => {
  const handler = handlers.get(request.type);
  if (handler) return await handler(request);
  return null;
};

// Export for other modules to check if a type is registered
export const isWriteType = (type: string): boolean => handlers.has(type);
