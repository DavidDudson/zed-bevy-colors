use bevy_color_lsp::server::run;
use tracing_subscriber::{fmt, EnvFilter};

fn main() {
    fmt()
        .with_env_filter(
            EnvFilter::try_from_env("BEVY_COLOR_LSP_LOG")
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();

    // Current-thread runtime keeps LSP stdio single-threaded; matches
    // the previous `#[tokio::main]` behavior.
    #[allow(clippy::expect_used)]
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio current-thread runtime must build at startup");
    rt.block_on(run());
}
