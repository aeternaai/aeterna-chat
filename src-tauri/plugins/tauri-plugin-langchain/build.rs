const COMMANDS: &[&str] = &[
    "spawn_langchain_service",
    "shutdown_langchain_service",
    "health_check",
    "ingest_document",
    "query_rag",
    "get_service_info",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .build();
}
