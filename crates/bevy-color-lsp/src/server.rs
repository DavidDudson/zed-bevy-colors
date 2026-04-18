//! Tower-LSP server implementation.

use crate::document::DocumentStore;
use crate::num::f32_to_u8_clamped;
use std::sync::Arc;
use tower_lsp::jsonrpc::Result;
#[allow(clippy::wildcard_imports)] // tower_lsp::lsp_types is a stable, well-scoped re-export
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tracing::instrument;

/// Tower-LSP backend holding shared document state.
#[derive(Debug)]
pub struct Backend {
    client: Client,
    docs: Arc<DocumentStore>,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self { client, docs: Arc::new(DocumentStore::default()) }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    #[instrument(skip(self), err)]
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                color_provider: Some(ColorProviderCapability::Simple(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "bevy-color-lsp".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
        })
    }

    #[instrument(skip(self))]
    async fn initialized(&self, _: InitializedParams) {
        self.client.log_message(MessageType::INFO, "bevy-color-lsp ready").await;
    }

    #[instrument(skip(self), err)]
    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    #[instrument(skip_all, fields(uri = %params.text_document.uri))]
    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.docs.open(params.text_document.uri, params.text_document.text);
    }

    #[instrument(skip_all, fields(uri = %params.text_document.uri))]
    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        for change in params.content_changes {
            match change.range {
                Some(range) => self.docs.apply_change(&uri, Some(range), &change.text),
                None => self.docs.replace(&uri, change.text),
            }
        }
    }

    #[instrument(skip_all, fields(uri = %params.text_document.uri))]
    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.docs.close(&params.text_document.uri);
    }

    #[instrument(skip_all, fields(uri = %params.text_document.uri), err)]
    async fn document_color(&self, params: DocumentColorParams) -> Result<Vec<ColorInformation>> {
        Ok(self
            .docs
            .colors_for(&params.text_document.uri)
            .into_iter()
            .map(|(range, m)| ColorInformation {
                range,
                color: Color {
                    red: m.color.r,
                    green: m.color.g,
                    blue: m.color.b,
                    alpha: m.color.a,
                },
            })
            .collect())
    }

    #[instrument(skip_all, err)]
    async fn color_presentation(
        &self,
        params: ColorPresentationParams,
    ) -> Result<Vec<ColorPresentation>> {
        let c = params.color;
        let r = f32_to_u8_clamped(c.red * 255.0);
        let g = f32_to_u8_clamped(c.green * 255.0);
        let b = f32_to_u8_clamped(c.blue * 255.0);
        let label = if c.alpha < 1.0 {
            format!("Color::srgba({:.3}, {:.3}, {:.3}, {:.3})", c.red, c.green, c.blue, c.alpha)
        } else {
            format!("Color::srgb_u8({r}, {g}, {b})")
        };
        Ok(vec![ColorPresentation { label, ..Default::default() }])
    }
}

/// Start the LSP server, reading from stdin and writing to stdout.
///
/// Blocks until the client closes the connection.
pub async fn run() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
