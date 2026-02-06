use thiserror::Error;

#[derive(Error, Debug)]
pub enum RendererError {
    #[error("wgpu error")]
    Wgpu(#[from] wgpu::RequestDeviceError),

    #[error("swapchain error: {0}")]
    Swapchain(String),
}
