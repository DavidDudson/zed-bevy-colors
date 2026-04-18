use bevy_color_lsp::server::run;

#[tokio::main]
async fn main() {
    run().await;
}
