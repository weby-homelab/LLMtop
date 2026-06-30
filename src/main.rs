//! Binary entry point. All logic lives in the `llmtop` library crate so it can
//! be reused in-process by other tools (see `src/lib.rs` and `src/snapshot.rs`).

fn main() -> std::io::Result<()> {
    llmtop::run()
}
