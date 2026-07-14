use std::sync::{Arc, Mutex};

use ironrdp_cliprdr::backend::{CliprdrBackend, CliprdrBackendFactory};
use ironrdp_cliprdr::pdu::{
    ClipboardFormat, ClipboardGeneralCapabilityFlags, FileContentsRequest, FileContentsResponse,
    FormatDataRequest, FormatDataResponse, LockDataId,
};

/// Clipboard backend using arboard for cross-platform clipboard access.
/// ponytail: text-only for now, add file copy when needed.
pub struct ClipboardBackend {
    clipboard: Arc<Mutex<arboard::Clipboard>>,
}

impl std::fmt::Debug for ClipboardBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClipboardBackend").finish_non_exhaustive()
    }
}

impl ClipboardBackend {
    pub fn new() -> Self {
        Self {
            clipboard: Arc::new(Mutex::new(arboard::Clipboard::new().unwrap())),
        }
    }
}

ironrdp_core::impl_as_any!(ClipboardBackend);

impl CliprdrBackend for ClipboardBackend {
    fn temporary_directory(&self) -> &str {
        "/tmp"
    }

    fn client_capabilities(&self) -> ClipboardGeneralCapabilityFlags {
        ClipboardGeneralCapabilityFlags::empty()
    }

    fn on_ready(&mut self) {
        tracing::debug!("Clipboard channel ready");
    }

    fn on_request_format_list(&mut self) {}

    fn on_process_negotiated_capabilities(&mut self, _caps: ClipboardGeneralCapabilityFlags) {}

    fn on_remote_copy(&mut self, available_formats: &[ClipboardFormat]) {
        tracing::debug!("Remote copy with {} formats", available_formats.len());
    }

    fn on_format_data_request(&mut self, request: FormatDataRequest) {
        let text = self
            .clipboard
            .lock()
            .unwrap()
            .get_text()
            .unwrap_or_default();
        tracing::debug!("Clipboard data request, sending {} bytes", text.len());
        let _ = request;
    }

    fn on_format_data_response(&mut self, response: FormatDataResponse<'_>) {
        if let Ok(text) = std::str::from_utf8(response.data()) {
            tracing::debug!("Received clipboard data: {} bytes", text.len());
            let _ = self.clipboard.lock().unwrap().set_text(text.to_owned());
        }
    }

    fn on_file_contents_request(&mut self, _request: FileContentsRequest) {}

    fn on_file_contents_response(&mut self, _response: FileContentsResponse<'_>) {}

    fn on_lock(&mut self, _data_id: LockDataId) {}

    fn on_unlock(&mut self, _data_id: LockDataId) {}
}

pub struct ClipboardBackendFactory;

impl CliprdrBackendFactory for ClipboardBackendFactory {
    fn build_cliprdr_backend(&self) -> Box<dyn CliprdrBackend> {
        Box::new(ClipboardBackend::new())
    }
}
