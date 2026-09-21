//! Screenshot of the monitor the mouse is on, for questions about what is on screen.
//!
//! The picture is taken with GDI and encoded as PNG by Windows Imaging (WIC), scaled down first:
//! a full 4K screen would cost a model many thousand tokens and most of them only accept a few MB.

use rust_i18n::t;
use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, GUID_ContainerFormatPng, GUID_WICPixelFormat32bppBGRA, IWICImagingFactory,
    WICBitmapEncoderNoCache, WICBitmapInterpolationModeFant,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, IStream, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
};

use windows::Win32::Foundation::HGLOBAL;
use windows::Win32::System::Com::StructuredStorage::CreateStreamOnHGlobal;
use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, GetMonitorInfoW, MonitorFromPoint,
    ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CAPTUREBLT, DIB_RGB_COLORS, HGDIOBJ, MONITORINFO,
    MONITOR_DEFAULTTOPRIMARY, SRCCOPY,
};
use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

/// Longest edge of the picture handed to the model. Text on screen stays readable at this size.
pub const MAX_EDGE: u32 = 1280;

pub struct Screenshot {
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// What the frontend gets: the picture as base64 PNG, ready for a preview and for the model.
/// Mirrored in `src/lib/ipc.ts` (`Screenshot`).
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotImage {
    /// Base64 PNG, without the `data:` prefix.
    pub data: String,
    pub width: u32,
    pub height: u32,
}

impl From<Screenshot> for ScreenshotImage {
    fn from(shot: Screenshot) -> Self {
        Self {
            data: to_base64(&shot.png),
            width: shot.width,
            height: shot.height,
        }
    }
}

/// Standard base64 with padding (RFC 4648). Small enough to keep here instead of adding a crate.
pub fn to_base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let bits = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        let index = |shift: u32| ALPHABET[((bits >> shift) & 0x3f) as usize] as char;
        out.push(index(18));
        out.push(index(12));
        out.push(if chunk.len() > 1 { index(6) } else { '=' });
        out.push(if chunk.len() > 2 { index(0) } else { '=' });
    }
    out
}

/// Grabs the monitor the mouse pointer is on.
pub fn monitor_at_cursor() -> Result<Screenshot, String> {
    let (pixels, width, height) = grab()?;
    let (target_width, target_height) = scaled(width, height, MAX_EDGE);
    let png = encode_png(&pixels, width, height, target_width, target_height)?;
    Ok(Screenshot {
        png,
        width: target_width,
        height: target_height,
    })
}

/// Raw BGRA pixels of the monitor under the cursor, top-down.
fn grab() -> Result<(Vec<u8>, u32, u32), String> {
    // SAFETY: Win32 calls with owned handles; every handle is released below, also on the error paths.
    unsafe {
        let mut cursor = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut cursor) == 0 {
            return Err(failed("GetCursorPos"));
        }
        let monitor = MonitorFromPoint(cursor, MONITOR_DEFAULTTOPRIMARY);
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return Err(failed("GetMonitorInfoW"));
        }
        let area = info.rcMonitor;
        let width = (area.right - area.left).max(1);
        let height = (area.bottom - area.top).max(1);

        let screen_dc = GetDC(std::ptr::null_mut());
        if screen_dc.is_null() {
            return Err(failed("GetDC"));
        }
        let memory_dc = CreateCompatibleDC(screen_dc);
        let mut header: BITMAPINFO = std::mem::zeroed();
        header.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            // Negative height: rows top-down, the order WIC expects.
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            ..std::mem::zeroed()
        };
        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
        let bitmap = CreateDIBSection(memory_dc, &header, DIB_RGB_COLORS, &mut bits, std::ptr::null_mut(), 0);

        let result = if bitmap.is_null() || bits.is_null() {
            Err(failed("CreateDIBSection"))
        } else {
            let previous = SelectObject(memory_dc, bitmap as HGDIOBJ);
            // CAPTUREBLT also catches layered windows (menus, tooltips).
            let copied = BitBlt(
                memory_dc,
                0,
                0,
                width,
                height,
                screen_dc,
                area.left,
                area.top,
                SRCCOPY | CAPTUREBLT,
            );
            let pixels = if copied == 0 {
                Err(failed("BitBlt"))
            } else {
                let len = (width as usize) * (height as usize) * 4;
                Ok(std::slice::from_raw_parts(bits as *const u8, len).to_vec())
            };
            SelectObject(memory_dc, previous);
            pixels.map(|pixels| (pixels, width as u32, height as u32))
        };

        if !bitmap.is_null() {
            DeleteObject(bitmap as HGDIOBJ);
        }
        DeleteDC(memory_dc);
        ReleaseDC(std::ptr::null_mut(), screen_dc);
        result
    }
}

/// Keeps the aspect ratio; never scales up.
fn scaled(width: u32, height: u32, max_edge: u32) -> (u32, u32) {
    let longest = width.max(height);
    if longest <= max_edge || longest == 0 {
        return (width, height);
    }
    let factor = f64::from(max_edge) / f64::from(longest);
    (
        ((f64::from(width) * factor).round() as u32).max(1),
        ((f64::from(height) * factor).round() as u32).max(1),
    )
}

fn encode_png(
    pixels: &[u8],
    width: u32,
    height: u32,
    target_width: u32,
    target_height: u32,
) -> Result<Vec<u8>, String> {
    // SAFETY: COM calls on interfaces created here; the pixel buffer outlives the source bitmap.
    unsafe {
        // The capture may run on any worker thread; WIC is happy in an apartment of its own.
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let factory: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).map_err(com("WIC factory"))?;

        let stride = width * 4;
        let source = factory
            .CreateBitmapFromMemory(width, height, &GUID_WICPixelFormat32bppBGRA, stride, pixels)
            .map_err(com("CreateBitmapFromMemory"))?;

        // A stream on global memory grows while the encoder writes; WIC's own memory stream is fixed-size.
        let stream: IStream =
            CreateStreamOnHGlobal(HGLOBAL(std::ptr::null_mut()), true).map_err(com("CreateStream"))?;

        let encoder = factory
            .CreateEncoder(&GUID_ContainerFormatPng, std::ptr::null())
            .map_err(com("CreateEncoder"))?;
        encoder
            .Initialize(&stream, WICBitmapEncoderNoCache)
            .map_err(com("encoder init"))?;

        let mut frame = None;
        encoder
            .CreateNewFrame(&mut frame, std::ptr::null_mut())
            .map_err(com("CreateNewFrame"))?;
        let frame = frame.ok_or_else(|| failed("CreateNewFrame"))?;
        frame.Initialize(None).map_err(com("frame init"))?;
        frame.SetSize(target_width, target_height).map_err(com("SetSize"))?;

        if (target_width, target_height) == (width, height) {
            frame
                .WriteSource(&source, std::ptr::null())
                .map_err(com("WriteSource"))?;
        } else {
            let scaler = factory.CreateBitmapScaler().map_err(com("CreateBitmapScaler"))?;
            scaler
                .Initialize(&source, target_width, target_height, WICBitmapInterpolationModeFant)
                .map_err(com("scaler init"))?;
            frame
                .WriteSource(&scaler, std::ptr::null())
                .map_err(com("WriteSource"))?;
        }
        frame.Commit().map_err(com("frame commit"))?;
        encoder.Commit().map_err(com("encoder commit"))?;

        read_stream(&stream)
    }
}

/// Copies what the encoder wrote into the memory stream.
unsafe fn read_stream(stream: &IStream) -> Result<Vec<u8>, String> {
    use windows::Win32::System::Com::{STREAM_SEEK_END, STREAM_SEEK_SET};

    let mut size = 0u64;
    stream.Seek(0, STREAM_SEEK_END, Some(&mut size)).map_err(com("seek"))?;
    stream.Seek(0, STREAM_SEEK_SET, None).map_err(com("seek"))?;
    let mut bytes = vec![0u8; size as usize];
    let mut read = 0u32;
    stream
        .Read(
            bytes.as_mut_ptr() as *mut std::ffi::c_void,
            size as u32,
            Some(&mut read),
        )
        .ok()
        .map_err(com("read"))?;
    bytes.truncate(read as usize);
    if bytes.is_empty() {
        return Err(failed("empty image"));
    }
    Ok(bytes)
}

fn failed(step: &str) -> String {
    t!("capture.failed", error = step).into()
}

fn com(step: &'static str) -> impl Fn(windows::core::Error) -> String {
    move |error| t!("capture.failed", error = format!("{step}: {error}")).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_the_standard() {
        assert_eq!(to_base64(b""), "");
        assert_eq!(to_base64(b"f"), "Zg==");
        assert_eq!(to_base64(b"fo"), "Zm8=");
        assert_eq!(to_base64(b"foo"), "Zm9v");
        assert_eq!(to_base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(to_base64(&[0xff, 0xef, 0xfe]), "/+/+");
    }

    #[test]
    fn scaling_keeps_the_aspect_ratio_and_never_grows() {
        assert_eq!(scaled(2560, 1440, 1280), (1280, 720));
        assert_eq!(scaled(1080, 1920, 1280), (720, 1280));
        assert_eq!(scaled(800, 600, 1280), (800, 600));
    }

    /// Takes a real screenshot of this machine. Run manually:
    /// `cargo test -- --ignored captures_the_screen`
    #[test]
    #[ignore = "captures the real screen"]
    fn captures_the_screen() {
        let shot = monitor_at_cursor().unwrap();
        assert_eq!(&shot.png[1..4], b"PNG");
        assert!(shot.width <= MAX_EDGE && shot.height <= MAX_EDGE);
        let path = std::env::temp_dir().join("cephi-capture-test.png");
        std::fs::write(&path, &shot.png).unwrap();
        println!("{} ({}x{})", path.display(), shot.width, shot.height);
    }
}
