#[path = "fixtures/mod.rs"]
mod fixtures;
#[path = "helpers/mod.rs"]
mod helpers;

#[path = "integration/agent_handoff.rs"]
mod agent_handoff;
#[path = "integration/graph_routing.rs"]
mod graph_routing;
#[path = "integration/pause_resume.rs"]
mod pause_resume;
#[path = "integration/permission_flow.rs"]
mod permission_flow;
#[path = "integration/tool_context_abort.rs"]
mod tool_context_abort;
#[path = "integration/tool_context_attachments.rs"]
mod tool_context_attachments;
#[path = "integration/tool_context_permission.rs"]
mod tool_context_permission;
#[path = "integration/workflow_run.rs"]
mod workflow_run;
