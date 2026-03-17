use chrono::Utc;
use criterion::{criterion_group, criterion_main, Criterion};
use std::sync::Arc;
use tmux_mcp_server::state::command_registry::CommandRegistry;
use tmux_mcp_server::tmux::models::{CommandExecution, CommandStatus};

fn create_test_execution(id: &str) -> CommandExecution {
    CommandExecution {
        id: id.to_string(),
        pane_id: "%0".to_string(),
        command: "echo test".to_string(),
        status: CommandStatus::Completed,
        start_time: Utc::now(),
        result: Some("test output".to_string()),
        exit_code: Some(0),
        raw_mode: false,
    }
}

fn bench_command_registry_insert(c: &mut Criterion) {
    c.bench_function("command_registry_insert_100", |b| {
        b.iter(|| {
            let registry = CommandRegistry::new(1000, 600);
            for i in 0..100 {
                let exec = create_test_execution(&format!("cmd-{}", i));
                registry.insert(format!("cmd-{}", i), exec);
            }
        });
    });

    c.bench_function("command_registry_insert_1000", |b| {
        b.iter(|| {
            let registry = CommandRegistry::new(10000, 600);
            for i in 0..1000 {
                let exec = create_test_execution(&format!("cmd-{}", i));
                registry.insert(format!("cmd-{}", i), exec);
            }
        });
    });
}

fn bench_command_registry_lookup(c: &mut Criterion) {
    let registry = Arc::new(CommandRegistry::new(1000, 600));

    // Populate registry
    for i in 0..1000 {
        let exec = create_test_execution(&format!("cmd-{}", i));
        registry.insert(format!("cmd-{}", i), exec);
    }

    c.bench_function("command_registry_lookup", |b| {
        let reg = registry.clone();
        b.iter(|| {
            // Look up random commands
            for i in 0..100 {
                let _ = reg.get(&format!("cmd-{}", i));
            }
        });
    });
}

criterion_group!(
    benches,
    bench_command_registry_insert,
    bench_command_registry_lookup
);
criterion_main!(benches);
