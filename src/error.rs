use thiserror::Error;

#[derive(Debug, Error)]
pub enum LibforgeError {
    #[error("renderer error: {0}")]
    Renderer(#[from] RendererError),

    #[error("platform error: {0}")]
    Platform(String),
}

#[derive(Error, Debug)]
pub enum RendererError {
    #[error("wgpu error")]
    Wgpu(#[from] wgpu::RequestDeviceError),

    #[error("surface error: {0}")]
    Surface(String),

    #[error("internal error: {0}")]
    Internal(String),
}
