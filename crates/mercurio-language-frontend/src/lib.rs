pub mod lowering;
pub mod resolver;
pub mod transpile;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SourceLanguage {
    Kerml,
    Sysml,
}

mod logging {
    #[cfg(not(target_arch = "wasm32"))]
    pub type CompileTimer = std::time::Instant;

    #[cfg(target_arch = "wasm32")]
    pub type CompileTimer = ();

    #[cfg(not(target_arch = "wasm32"))]
    pub fn compile_timer_start() -> CompileTimer {
        std::time::Instant::now()
    }

    #[cfg(target_arch = "wasm32")]
    pub fn compile_timer_start() -> CompileTimer {}

    pub fn log_compile_timed_event(
        _operation: &str,
        _start: CompileTimer,
        _outcome: &str,
        _details: impl AsRef<str>,
    ) {
    }
}
