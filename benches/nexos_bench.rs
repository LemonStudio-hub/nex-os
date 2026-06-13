//! Performance benchmarks for NexOS core subsystems.
//!
//! Run with: `cargo bench --target x86_64-unknown-linux-gnu`
//!
//! Benchmark groups:
//! - **VFS core** — path resolution, directory/file CRUD, deep nesting
//! - **ChunkedContent** — construction, line counting, range reads, appending
//! - **Serialization** — JSON roundtrip for VFS and ShellState
//! - **Command execution** — end-to-end dispatch through the command layer
//! - **Shell infrastructure** — pipe splitting, redirect extraction, registry lookup

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use nexos::command::Registry;
use nexos::shell::{Service, ShellState};
use nexos::vfs::{ChunkedContent, Vfs};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a fresh Service + VFS with a small set of pre-populated files.
fn setup() -> (Service, ShellState) {
    let service = Service::new();
    let mut vfs = Vfs::new();
    vfs.write_file("/home/user/hello.txt", "Hello, World!")
        .unwrap();
    vfs.write_file("/home/user/lines.txt", "line1\nline2\nline3\nline4\nline5\n")
        .unwrap();
    vfs.mkdir("/home/user/subdir").unwrap();
    vfs.write_file("/home/user/subdir/nested.txt", "nested content")
        .unwrap();
    let state = ShellState::new(vfs);
    (service, state)
}

/// Create a VFS populated with `n` files in a flat directory.
fn setup_large_vfs(n: usize) -> Vfs {
    let mut vfs = Vfs::new();
    vfs.mkdir("/home/user/bench").unwrap();
    for i in 0..n {
        vfs.write_file(
            &format!("/home/user/bench/file_{:04}.txt", i),
            &format!("content of file {}\n", i),
        )
        .unwrap();
    }
    vfs
}

/// Generate a multi-line text block with `n` lines.
fn generate_lines(n: usize) -> String {
    (0..n)
        .map(|i| format!("This is line number {} with some padding text.", i))
        .collect::<Vec<_>>()
        .join("\n")
}

// ===========================================================================
// Group 1: VFS Core Operations
// ===========================================================================

fn bench_vfs(c: &mut Criterion) {
    let mut group = c.benchmark_group("vfs");

    // --- resolve_path ---
    group.bench_function("resolve_path_absolute", |b| {
        let vfs = Vfs::new();
        b.iter(|| black_box(vfs.resolve_path("/home/user/subdir/file.txt").unwrap()));
    });

    group.bench_function("resolve_path_tilde", |b| {
        let vfs = Vfs::new();
        b.iter(|| black_box(vfs.resolve_path("~/Documents/projects").unwrap()));
    });

    group.bench_function("resolve_path_relative_with_dotdot", |b| {
        let mut vfs = Vfs::new();
        vfs.cwd = "/home/user/subdir".to_string();
        b.iter(|| black_box(vfs.resolve_path("../../etc/../tmp").unwrap()));
    });

    // --- mkdir + touch ---
    let mut counter = 0usize;
    group.bench_function("mkdir_and_touch", |b| {
        let mut vfs = Vfs::new();
        counter = 0;
        b.iter(|| {
            let path = format!("/home/user/bench_{}", counter);
            vfs.mkdir(&path).unwrap();
            vfs.touch(&format!("{}/file.txt", path)).unwrap();
            counter += 1;
            black_box(());
        });
    });

    // --- write_file / read_file roundtrip ---
    for size in [64, 1024, 16384, 131072] {
        group.bench_with_input(
            BenchmarkId::new("write_read_roundtrip", format!("{}B", size)),
            &size,
            |b, &size| {
                let mut vfs = Vfs::new();
                let content = "x".repeat(size);
                b.iter(|| {
                    vfs.write_file("/home/user/bench.txt", &content).unwrap();
                    black_box(vfs.read_file("/home/user/bench.txt").unwrap());
                });
            },
        );
    }

    // --- list_dir at various scales ---
    for n in [10, 100, 1000] {
        group.bench_with_input(
            BenchmarkId::new("list_dir", format!("{}files", n)),
            &n,
            |b, &n| {
                let vfs = setup_large_vfs(n);
                b.iter(|| black_box(vfs.list_dir("/home/user/bench").unwrap()));
            },
        );
    }

    // --- deep nesting ---
    group.bench_function("deep_nesting_mkdir_read", |b| {
        let mut vfs = Vfs::new();
        let mut path = String::from("/home/user");
        for i in 0..20 {
            path.push_str(&format!("/level_{}", i));
            vfs.mkdir(&path).unwrap();
        }
        let deep_file = format!("{}/deep.txt", path);
        vfs.write_file(&deep_file, "deep content").unwrap();
        b.iter(|| {
            black_box(vfs.resolve_path(&deep_file).unwrap());
            black_box(vfs.read_file(&deep_file).unwrap());
        });
    });

    // --- cp / mv ---
    group.bench_function("cp_file", |b| {
        let mut vfs = Vfs::new();
        vfs.write_file("/home/user/src.txt", "copy me").unwrap();
        let mut i = 0usize;
        b.iter(|| {
            let dst = format!("/home/user/copy_{}.txt", i);
            vfs.cp("/home/user/src.txt", &dst).unwrap();
            i += 1;
            black_box(());
        });
    });

    group.finish();
}

// ===========================================================================
// Group 2: ChunkedContent
// ===========================================================================

fn bench_chunked_content(c: &mut Criterion) {
    let mut group = c.benchmark_group("chunked_content");

    // --- from_string ---
    group.bench_function("from_string_small_1KB", |b| {
        let s = "a".repeat(1024);
        b.iter(|| black_box(ChunkedContent::from_string(black_box(&s))));
    });

    group.bench_function("from_string_medium_64KB", |b| {
        let s = "b".repeat(65_536);
        b.iter(|| black_box(ChunkedContent::from_string(black_box(&s))));
    });

    group.bench_function("from_string_large_256KB", |b| {
        let s = "c".repeat(262_144);
        b.iter(|| black_box(ChunkedContent::from_string(black_box(&s))));
    });

    // --- as_string (concatenation) ---
    group.bench_function("as_string_small", |b| {
        let c = ChunkedContent::from_string(&"x".repeat(1024));
        b.iter(|| black_box(c.as_string()));
    });

    group.bench_function("as_string_large_256KB", |b| {
        let c = ChunkedContent::from_string(&"x".repeat(262_144));
        b.iter(|| black_box(c.as_string()));
    });

    // --- line_count ---
    group.bench_function("line_count_1000_lines", |b| {
        let c = ChunkedContent::from_string(&generate_lines(1000));
        b.iter(|| black_box(c.line_count()));
    });

    group.bench_function("line_count_10000_lines", |b| {
        let c = ChunkedContent::from_string(&generate_lines(10_000));
        b.iter(|| black_box(c.line_count()));
    });

    // --- lines() range extraction ---
    group.bench_function("lines_first_10_of_10000", |b| {
        let c = ChunkedContent::from_string(&generate_lines(10_000));
        b.iter(|| black_box(c.lines(0, 10)));
    });

    group.bench_function("lines_middle_100_of_10000", |b| {
        let c = ChunkedContent::from_string(&generate_lines(10_000));
        b.iter(|| black_box(c.lines(5000, 100)));
    });

    // --- push_str ---
    group.bench_function("push_str_incremental_100x", |b| {
        b.iter(|| {
            let mut c = ChunkedContent::new();
            for i in 0..100 {
                c.push_str(&format!("line {}\n", i));
            }
            black_box(c);
        });
    });

    group.finish();
}

// ===========================================================================
// Group 3: Serialization (JSON roundtrip)
// ===========================================================================

fn bench_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization");

    // --- VFS to_json ---
    group.bench_function("vfs_to_json_small", |b| {
        let vfs = setup_large_vfs(5);
        b.iter(|| black_box(vfs.to_json()));
    });

    group.bench_function("vfs_to_json_100_files", |b| {
        let vfs = setup_large_vfs(100);
        b.iter(|| black_box(vfs.to_json()));
    });

    group.bench_function("vfs_to_json_1000_files", |b| {
        let vfs = setup_large_vfs(1000);
        b.iter(|| black_box(vfs.to_json()));
    });

    // --- VFS from_json ---
    group.bench_function("vfs_from_json_small", |b| {
        let vfs = setup_large_vfs(5);
        let json = vfs.to_json();
        b.iter(|| black_box(Vfs::from_json(black_box(&json)).unwrap()));
    });

    group.bench_function("vfs_from_json_1000_files", |b| {
        let vfs = setup_large_vfs(1000);
        let json = vfs.to_json();
        b.iter(|| black_box(Vfs::from_json(black_box(&json)).unwrap()));
    });

    // --- to_tree_json (skeleton only) ---
    group.bench_function("vfs_to_tree_json_1000_files", |b| {
        let vfs = setup_large_vfs(1000);
        b.iter(|| black_box(vfs.to_tree_json()));
    });

    // --- ShellState full roundtrip ---
    group.bench_function("shell_state_roundtrip", |b| {
        let (_, mut state) = setup();
        for i in 0..50 {
            state.history.push(format!("command_{}", i));
        }
        let json = state.to_json();
        b.iter(|| {
            let restored = ShellState::from_state_json(&json, "user").unwrap();
            black_box(restored);
        });
    });

    group.finish();
}

// ===========================================================================
// Group 4: Command Execution (end-to-end)
// ===========================================================================

fn bench_commands(c: &mut Criterion) {
    let mut group = c.benchmark_group("commands");

    // --- echo (measures pure dispatch overhead) ---
    group.bench_function("echo", |b| {
        let (service, mut state) = setup();
        b.iter(|| {
            let out = service.execute_command(&mut state, "echo hello world", None);
            black_box(out.stdout);
        });
    });

    // --- cat ---
    group.bench_function("cat", |b| {
        let (service, mut state) = setup();
        b.iter(|| {
            let out = service.execute_command(&mut state, "cat /home/user/hello.txt", None);
            black_box(out.stdout);
        });
    });

    // --- ls ---
    group.bench_function("ls", |b| {
        let service = Service::new();
        let vfs = setup_large_vfs(50);
        let mut state = ShellState::new(vfs);
        b.iter(|| {
            let out = service.execute_command(&mut state, "ls /home/user/bench", None);
            black_box(out.stdout);
        });
    });

    // --- mkdir + touch ---
    let mut cmd_counter = 0usize;
    group.bench_function("mkdir_touch", |b| {
        let service = Service::new();
        let vfs = Vfs::new();
        let mut state = ShellState::new(vfs);
        cmd_counter = 0;
        b.iter(|| {
            let dir = format!("/home/user/d_{}", cmd_counter);
            service.execute_command(&mut state, &format!("mkdir {}", dir), None);
            let out = service.execute_command(&mut state, &format!("touch {}/f.txt", dir), None);
            black_box(out.stdout);
            cmd_counter += 1;
        });
    });

    // --- grep ---
    group.bench_function("grep", |b| {
        let service = Service::new();
        let mut vfs = Vfs::new();
        let lines = generate_lines(500);
        vfs.write_file("/home/user/big.txt", &lines).unwrap();
        let mut state = ShellState::new(vfs);
        b.iter(|| {
            let out = service.execute_command(&mut state, "grep line /home/user/big.txt", None);
            black_box(out.stdout);
        });
    });

    // --- sort ---
    group.bench_function("sort_500_lines", |b| {
        let service = Service::new();
        let mut vfs = Vfs::new();
        let lines = generate_lines(500);
        vfs.write_file("/home/user/unsorted.txt", &lines).unwrap();
        let mut state = ShellState::new(vfs);
        b.iter(|| {
            let out = service.execute_command(&mut state, "sort /home/user/unsorted.txt", None);
            black_box(out.stdout);
        });
    });

    // --- wc ---
    group.bench_function("wc", |b| {
        let service = Service::new();
        let mut vfs = Vfs::new();
        let lines = generate_lines(1000);
        vfs.write_file("/home/user/big.txt", &lines).unwrap();
        let mut state = ShellState::new(vfs);
        b.iter(|| {
            let out = service.execute_command(&mut state, "wc /home/user/big.txt", None);
            black_box(out.stdout);
        });
    });

    // --- head / tail ---
    group.bench_function("head", |b| {
        let service = Service::new();
        let mut vfs = Vfs::new();
        let lines = generate_lines(5000);
        vfs.write_file("/home/user/big.txt", &lines).unwrap();
        let mut state = ShellState::new(vfs);
        b.iter(|| {
            let out = service.execute_command(&mut state, "head /home/user/big.txt", None);
            black_box(out.stdout);
        });
    });

    group.bench_function("tail", |b| {
        let service = Service::new();
        let mut vfs = Vfs::new();
        let lines = generate_lines(5000);
        vfs.write_file("/home/user/big.txt", &lines).unwrap();
        let mut state = ShellState::new(vfs);
        b.iter(|| {
            let out = service.execute_command(&mut state, "tail /home/user/big.txt", None);
            black_box(out.stdout);
        });
    });

    // --- pipe chain: cat | grep | wc ---
    group.bench_function("pipe_cat_grep_wc", |b| {
        let service = Service::new();
        let mut vfs = Vfs::new();
        let lines = generate_lines(500);
        vfs.write_file("/home/user/data.txt", &lines).unwrap();
        let mut state = ShellState::new(vfs);
        b.iter(|| {
            let out = service.execute_command(
                &mut state,
                "cat /home/user/data.txt | grep line | wc -l",
                None,
            );
            black_box(out.stdout);
        });
    });

    // --- && chaining ---
    group.bench_function("and_chain_3_commands", |b| {
        let (service, mut state) = setup();
        b.iter(|| {
            let out = service.execute_command(
                &mut state,
                "echo hello && cat /home/user/hello.txt && pwd",
                None,
            );
            black_box(out.stdout);
        });
    });

    // --- find ---
    group.bench_function("find", |b| {
        let service = Service::new();
        let vfs = setup_large_vfs(100);
        let mut state = ShellState::new(vfs);
        b.iter(|| {
            let out = service.execute_command(&mut state, "find /home/user/bench", None);
            black_box(out.stdout);
        });
    });

    group.finish();
}

// ===========================================================================
// Group 5: Shell Infrastructure
// ===========================================================================

fn bench_shell_infra(c: &mut Criterion) {
    let mut group = c.benchmark_group("shell_infra");

    // --- pipeline splitting ---
    group.bench_function("split_pipe_stages_simple", |b| {
        b.iter(|| {
            black_box(nexos::shell::pipeline::split_pipe_stages(
                "cat file | grep hello | wc -l",
            ))
        });
    });

    group.bench_function("split_pipe_stages_quoted", |b| {
        b.iter(|| {
            black_box(nexos::shell::pipeline::split_pipe_stages(
                r#"echo 'hello|world' | grep "foo|bar" | wc"#,
            ))
        });
    });

    // --- redirect extraction ---
    group.bench_function("extract_redirect_none", |b| {
        b.iter(|| black_box(nexos::shell::pipeline::extract_redirect("cat file.txt")));
    });

    group.bench_function("extract_redirect_overwrite", |b| {
        b.iter(|| {
            black_box(nexos::shell::pipeline::extract_redirect(
                "echo hello > output.txt",
            ))
        });
    });

    group.bench_function("extract_redirect_append", |b| {
        b.iter(|| {
            black_box(nexos::shell::pipeline::extract_redirect(
                "echo hello >> output.txt",
            ))
        });
    });

    group.bench_function("extract_redirect_no_space", |b| {
        b.iter(|| {
            black_box(nexos::shell::pipeline::extract_redirect(
                "echo hello>output.txt",
            ))
        });
    });

    // --- registry lookup ---
    group.bench_function("registry_lookup_existing", |b| {
        let registry = Registry::new();
        b.iter(|| black_box(registry.get("cat")));
    });

    group.bench_function("registry_lookup_nonexistent", |b| {
        let registry = Registry::new();
        b.iter(|| black_box(registry.get("nonexistent_command")));
    });

    group.bench_function("registry_completions", |b| {
        let registry = Registry::new();
        b.iter(|| black_box(registry.completions("c")));
    });

    group.bench_function("registry_completions_narrow", |b| {
        let registry = Registry::new();
        b.iter(|| black_box(registry.completions("chm")));
    });

    group.finish();
}

// ===========================================================================
// Criterion harness
// ===========================================================================

criterion_group!(
    benches,
    bench_vfs,
    bench_chunked_content,
    bench_serialization,
    bench_commands,
    bench_shell_infra,
);
criterion_main!(benches);
