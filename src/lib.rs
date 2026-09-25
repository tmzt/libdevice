//! Deterministic device setup for the desktop-backed mobile simulator.
//!
//! The first milestone intentionally renders offscreen. That keeps the device
//! contract independent of a windowing toolkit while matching the target
//! portrait geometry used by the eventual touch overlay.

use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

/// Portrait dimensions used by the Samsung Galaxy S24 simulator target.
pub const GALAXY_S24_PORTRAIT: PortraitGeometry = PortraitGeometry {
    width: 1080,
    height: 2340,
};

/// Pixel dimensions for the simulated device surface.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortraitGeometry {
    pub width: u32,
    pub height: u32,
}

impl PortraitGeometry {
    #[must_use]
    pub const fn extent(self) -> wgpu::Extent3d {
        wgpu::Extent3d {
            width: self.width,
            height: self.height,
            depth_or_array_layers: 1,
        }
    }
}

/// Parameters controlling adapter selection and the simulated display.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeviceConfig {
    pub geometry: PortraitGeometry,
    pub force_fallback_adapter: bool,
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            geometry: GALAXY_S24_PORTRAIT,
            force_fallback_adapter: false,
        }
    }
}

/// Errors raised while acquiring a GPU adapter or logical device.
#[derive(Debug)]
pub enum DeviceError {
    AdapterUnavailable,
    RequestDevice(wgpu::RequestDeviceError),
}

impl fmt::Display for DeviceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AdapterUnavailable => {
                formatter.write_str("no compatible GPU adapter is available")
            }
            Self::RequestDevice(error) => {
                write!(formatter, "failed to request GPU device: {error}")
            }
        }
    }
}

impl std::error::Error for DeviceError {}

/// Shared GPU handles and geometry for the simulator.
pub struct Device {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    geometry: PortraitGeometry,
}

impl Device {
    /// Acquire a GPU-backed device using the supplied simulator configuration.
    pub async fn new(config: DeviceConfig) -> Result<Self, DeviceError> {
        let mut instance_descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        instance_descriptor.backends = wgpu::Backends::PRIMARY;
        let instance = wgpu::Instance::new(instance_descriptor);
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: config.force_fallback_adapter,
                apply_limit_buckets: false,
            })
            .await
            .map_err(|_| DeviceError::AdapterUnavailable)?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("sigilweaver-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                memory_hints: wgpu::MemoryHints::Performance,
                ..Default::default()
            })
            .await
            .map_err(DeviceError::RequestDevice)?;

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            geometry: config.geometry,
        })
    }

    /// Synchronous convenience constructor for native examples and tests.
    pub fn new_blocking(config: DeviceConfig) -> Result<Self, DeviceError> {
        block_on(Self::new(config))
    }

    #[must_use]
    pub const fn geometry(&self) -> PortraitGeometry {
        self.geometry
    }

    #[must_use]
    pub fn adapter_info(&self) -> wgpu::AdapterInfo {
        self.adapter.get_info()
    }

    #[must_use]
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    #[must_use]
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Keep the instance owned by the device for the lifetime of all resources.
    #[must_use]
    pub fn instance(&self) -> &wgpu::Instance {
        &self.instance
    }

    /// Allocate an offscreen RGBA target matching the simulated portrait.
    #[must_use]
    pub fn create_color_target(&self, label: &'static str) -> wgpu::Texture {
        self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: self.geometry.extent(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
    }
}

fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = Pin::from(Box::new(future));
    let waker = noop_waker();
    let mut context = Context::from_waker(&waker);

    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

fn noop_waker() -> Waker {
    // The wgpu futures make progress when polled and do not require a wake
    // queue for this small synchronous entry point.
    unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &NOOP_WAKER_VTABLE)) }
}

const NOOP_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
    |_| RawWaker::new(std::ptr::null(), &NOOP_WAKER_VTABLE),
    |_| {},
    |_| {},
    |_| {},
);

#[cfg(test)]
mod tests {
    use super::{DeviceConfig, GALAXY_S24_PORTRAIT, PortraitGeometry};

    #[test]
    fn default_geometry_matches_s24_portrait() {
        assert_eq!(DeviceConfig::default().geometry, GALAXY_S24_PORTRAIT);
        assert_eq!(GALAXY_S24_PORTRAIT.extent().depth_or_array_layers, 1);
    }

    #[test]
    fn custom_geometry_produces_matching_extent() {
        let geometry = PortraitGeometry {
            width: 320,
            height: 640,
        };
        assert_eq!(geometry.extent().width, 320);
        assert_eq!(geometry.extent().height, 640);
    }
}
