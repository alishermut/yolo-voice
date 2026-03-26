/// Focused-window and full-screen screenshot capture.
/// Returns JPEG-encoded bytes. Never writes to disk.

/// Capture a screenshot of the focused window identified by its handle.
#[cfg(windows)]
pub fn capture_focused_window(hwnd: isize) -> Result<Vec<u8>, String> {
    use image::{ImageBuffer, RgbaImage};
    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
        GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        SRCCOPY,
    };
    use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

    unsafe {
        let hwnd = HWND(hwnd as *mut _);
        let mut rect = RECT::default();
        GetClientRect(hwnd, &mut rect)
            .map_err(|e| format!("GetClientRect failed: {}", e))?;

        let width = (rect.right - rect.left) as i32;
        let height = (rect.bottom - rect.top) as i32;
        if width <= 0 || height <= 0 {
            return Err("Window has zero size".to_string());
        }

        let hdc_window = GetDC(hwnd);
        if hdc_window.is_invalid() {
            return Err("GetDC failed".to_string());
        }

        let hdc_mem = CreateCompatibleDC(hdc_window);
        if hdc_mem.is_invalid() {
            ReleaseDC(hwnd, hdc_window);
            return Err("CreateCompatibleDC failed".to_string());
        }

        let hbm = CreateCompatibleBitmap(hdc_window, width, height);
        if hbm.is_invalid() {
            let _ = DeleteDC(hdc_mem);
            let _ = ReleaseDC(hwnd, hdc_window);
            return Err("CreateCompatibleBitmap failed".to_string());
        }

        let old_bm = SelectObject(hdc_mem, hbm);
        let blt_ok = BitBlt(hdc_mem, 0, 0, width, height, hdc_window, 0, 0, SRCCOPY);

        if blt_ok.is_err() {
            let _ = SelectObject(hdc_mem, old_bm);
            let _ = DeleteObject(hbm);
            let _ = DeleteDC(hdc_mem);
            let _ = ReleaseDC(hwnd, hdc_window);
            return Err("BitBlt failed".to_string());
        }

        // Extract pixel data
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [Default::default()],
        };

        let mut pixels = vec![0u8; (width * height * 4) as usize];
        let lines = GetDIBits(
            hdc_mem,
            hbm,
            0,
            height as u32,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        // Cleanup GDI
        let _ = SelectObject(hdc_mem, old_bm);
        let _ = DeleteObject(hbm);
        let _ = DeleteDC(hdc_mem);
        let _ = ReleaseDC(hwnd, hdc_window);

        if lines == 0 {
            return Err("GetDIBits returned 0 lines".to_string());
        }

        // Convert BGRA → RGBA
        for chunk in pixels.chunks_exact_mut(4) {
            chunk.swap(0, 2); // B ↔ R
        }

        let img: RgbaImage =
            ImageBuffer::from_raw(width as u32, height as u32, pixels)
                .ok_or_else(|| "Failed to build image buffer".to_string())?;

        encode_jpeg(&img, 85)
    }
}

/// Capture the full primary screen.
#[cfg(windows)]
pub fn capture_full_screen() -> Result<Vec<u8>, String> {
    use image::{ImageBuffer, RgbaImage};
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
        GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        SRCCOPY,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

    unsafe {
        let width = GetSystemMetrics(SM_CXSCREEN);
        let height = GetSystemMetrics(SM_CYSCREEN);
        if width <= 0 || height <= 0 {
            return Err("Screen metrics returned zero".to_string());
        }

        let hdc_screen = GetDC(None);
        if hdc_screen.is_invalid() {
            return Err("GetDC(null) failed".to_string());
        }

        let hdc_mem = CreateCompatibleDC(hdc_screen);
        if hdc_mem.is_invalid() {
            let _ = ReleaseDC(None, hdc_screen);
            return Err("CreateCompatibleDC failed".to_string());
        }

        let hbm = CreateCompatibleBitmap(hdc_screen, width, height);
        if hbm.is_invalid() {
            let _ = DeleteDC(hdc_mem);
            let _ = ReleaseDC(None, hdc_screen);
            return Err("CreateCompatibleBitmap failed".to_string());
        }

        let old_bm = SelectObject(hdc_mem, hbm);

        if let Err(e) = BitBlt(hdc_mem, 0, 0, width, height, hdc_screen, 0, 0, SRCCOPY) {
            let _ = SelectObject(hdc_mem, old_bm);
            let _ = DeleteObject(hbm);
            let _ = DeleteDC(hdc_mem);
            let _ = ReleaseDC(None, hdc_screen);
            return Err(format!("BitBlt failed: {}", e));
        }

        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [Default::default()],
        };

        let mut pixels = vec![0u8; (width * height * 4) as usize];
        let lines = GetDIBits(
            hdc_mem,
            hbm,
            0,
            height as u32,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        let _ = SelectObject(hdc_mem, old_bm);
        let _ = DeleteObject(hbm);
        let _ = DeleteDC(hdc_mem);
        let _ = ReleaseDC(None, hdc_screen);

        if lines == 0 {
            return Err("GetDIBits returned 0 lines".to_string());
        }

        for chunk in pixels.chunks_exact_mut(4) {
            chunk.swap(0, 2);
        }

        let img: RgbaImage =
            ImageBuffer::from_raw(width as u32, height as u32, pixels)
                .ok_or_else(|| "Failed to build image buffer".to_string())?;

        encode_jpeg(&img, 85)
    }
}

// ── macOS implementation via Core Graphics ──────────────────────────────────

#[cfg(target_os = "macos")]
pub fn capture_focused_window(pid: isize) -> Result<Vec<u8>, String> {
    use core_graphics::display::*;
    use core_graphics::geometry::{CGRect, CGSize, CGPoint};
    use core_graphics::window::*;
    use core_foundation::array::CFArray;
    use core_foundation::base::TCFType;
    use core_foundation::dictionary::CFDictionaryRef;
    use core_foundation::number::CFNumber;
    use core_foundation::string::CFString;

    // List all on-screen windows and find one matching the target PID
    let window_list = unsafe {
        CGWindowListCopyWindowInfo(
            kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements,
            kCGNullWindowID,
        )
    };

    if window_list.is_null() {
        return Err("Failed to get window list".to_string());
    }

    let windows: CFArray = unsafe { TCFType::wrap_under_get_rule(window_list) };
    let mut target_window_id: Option<u32> = None;

    for i in 0..windows.len() {
        let dict = unsafe { *windows.get_unchecked(i) } as CFDictionaryRef;
        if dict.is_null() {
            continue;
        }

        // Get the owner PID
        let pid_key = CFString::new("kCGWindowOwnerPID");
        let mut pid_value: *const core_foundation::base::CFType = std::ptr::null();
        let has_pid = unsafe {
            core_foundation::dictionary::CFDictionaryGetValueIfPresent(
                dict,
                pid_key.as_concrete_TypeRef() as *const _,
                &mut pid_value as *mut _ as *mut *const _,
            )
        };

        if has_pid == 0 || pid_value.is_null() {
            continue;
        }

        let cf_pid: CFNumber = unsafe { TCFType::wrap_under_get_rule(pid_value as *const _) };
        if let Some(window_pid) = cf_pid.to_i64() {
            if window_pid == pid as i64 {
                // Get the window ID
                let id_key = CFString::new("kCGWindowNumber");
                let mut id_value: *const core_foundation::base::CFType = std::ptr::null();
                let has_id = unsafe {
                    core_foundation::dictionary::CFDictionaryGetValueIfPresent(
                        dict,
                        id_key.as_concrete_TypeRef() as *const _,
                        &mut id_value as *mut _ as *mut *const _,
                    )
                };

                if has_id != 0 && !id_value.is_null() {
                    let cf_id: CFNumber =
                        unsafe { TCFType::wrap_under_get_rule(id_value as *const _) };
                    if let Some(wid) = cf_id.to_i64() {
                        target_window_id = Some(wid as u32);
                        break;
                    }
                }
            }
        }
    }

    let window_id = target_window_id
        .ok_or_else(|| "No on-screen window found for the given PID".to_string())?;

    // Capture the specific window
    let cg_image = unsafe {
        CGWindowListCreateImage(
            CGRect {
                origin: CGPoint { x: 0.0, y: 0.0 },
                size: CGSize {
                    width: 0.0,
                    height: 0.0,
                },
            },
            kCGWindowListOptionIncludingWindow,
            window_id,
            kCGWindowImageBoundsIgnoreFraming,
        )
    };

    if cg_image.is_null() {
        return Err("CGWindowListCreateImage returned null".to_string());
    }

    cg_image_to_jpeg(unsafe { &*cg_image })
}

#[cfg(target_os = "macos")]
pub fn capture_full_screen() -> Result<Vec<u8>, String> {
    use core_graphics::display::*;
    use core_graphics::geometry::{CGRect, CGSize, CGPoint};

    let cg_image = unsafe {
        CGWindowListCreateImage(
            CGRect {
                origin: CGPoint { x: 0.0, y: 0.0 },
                size: CGSize {
                    width: 0.0,
                    height: 0.0,
                },
            },
            kCGWindowListOptionOnScreenOnly,
            kCGNullWindowID,
            kCGWindowImageDefault,
        )
    };

    if cg_image.is_null() {
        return Err("CGWindowListCreateImage returned null for full screen".to_string());
    }

    cg_image_to_jpeg(unsafe { &*cg_image })
}

/// Convert a CGImage to JPEG bytes via the shared encode_jpeg helper.
#[cfg(target_os = "macos")]
fn cg_image_to_jpeg(cg_image: &core_graphics::image::CGImage) -> Result<Vec<u8>, String> {
    use image::{ImageBuffer, RgbaImage};

    let width = cg_image.width();
    let height = cg_image.height();
    let bytes_per_row = cg_image.bytes_per_row();
    let data = cg_image.data();
    let raw_bytes = data.bytes();

    if width == 0 || height == 0 {
        return Err("CGImage has zero dimensions".to_string());
    }

    // CGImage data is typically BGRA or RGBA depending on color space.
    // Core Graphics on macOS uses premultiplied BGRA by default.
    let mut rgba = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        let row_start = y * bytes_per_row;
        for x in 0..width {
            let offset = row_start + x * 4;
            if offset + 3 < raw_bytes.len() {
                // BGRA → RGBA
                rgba.push(raw_bytes[offset + 2]); // R
                rgba.push(raw_bytes[offset + 1]); // G
                rgba.push(raw_bytes[offset]);     // B
                rgba.push(raw_bytes[offset + 3]); // A
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 255]);
            }
        }
    }

    let img: RgbaImage = ImageBuffer::from_raw(width as u32, height as u32, rgba)
        .ok_or_else(|| "Failed to build image buffer from CGImage".to_string())?;

    encode_jpeg(&img, 85)
}

// ── JPEG encoding ───────────────────────────────────────────────────────────

fn encode_jpeg(img: &image::RgbaImage, quality: u8) -> Result<Vec<u8>, String> {
    use image::ImageEncoder;
    // JPEG requires RGB (no alpha). Screenshots have opaque pixels so this is safe.
    let rgb_img: image::RgbImage = image::DynamicImage::ImageRgba8(img.clone()).into_rgb8();
    let mut buf = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality);
    encoder
        .write_image(
            rgb_img.as_raw(),
            rgb_img.width(),
            rgb_img.height(),
            image::ExtendedColorType::Rgb8,
        )
        .map_err(|e| format!("JPEG encode error: {}", e))?;
    Ok(buf)
}

#[allow(dead_code)]
fn encode_png(img: &image::RgbaImage) -> Result<Vec<u8>, String> {
    use image::ImageEncoder;
    let mut buf = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut buf);
    encoder
        .write_image(
            img.as_raw(),
            img.width(),
            img.height(),
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| format!("PNG encode error: {}", e))?;
    Ok(buf)
}
