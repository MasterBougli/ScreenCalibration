//! Détection des écrans : résolution, fréquence, profondeur de couleur, HDR,
//! primaires/luminances annoncées (EDID via DXGI), niveau de blanc SDR et profil ICC.

use windows::Win32::Devices::Display::*;
use windows::Win32::Foundation::{ERROR_SUCCESS, RECT};
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::ColorSystem::GetICMProfileW;
use windows::Win32::UI::WindowsAndMessaging::MONITORINFOF_PRIMARY;
use windows::core::{Interface, PCWSTR, PWSTR, w};

#[derive(Clone, Debug, Default)]
pub struct MonitorInfo {
    pub gdi_name: String,
    pub friendly_name: String,
    pub adapter_name: String,
    pub rect: RECT,
    pub refresh_hz: u32,
    pub primary: bool,
    pub bits_per_color: u32,
    pub hdr_active: bool,
    pub color_space: String,
    pub advanced_color_supported: bool,
    pub advanced_color_enabled: bool,
    pub wide_color_enforced: bool,
    pub advanced_color_force_disabled: bool,
    pub red_primary: [f32; 2],
    pub green_primary: [f32; 2],
    pub blue_primary: [f32; 2],
    pub white_point: [f32; 2],
    pub min_luminance: f32,
    pub max_luminance: f32,
    pub max_full_frame_luminance: f32,
    pub sdr_white_nits: Option<f32>,
    pub icc_profile: Option<String>,
}

impl MonitorInfo {
    pub fn width(&self) -> i32 {
        self.rect.right - self.rect.left
    }

    pub fn height(&self) -> i32 {
        self.rect.bottom - self.rect.top
    }

    pub fn label(&self) -> String {
        let name = if self.friendly_name.is_empty() { &self.gdi_name } else { &self.friendly_name };
        format!(
            "{}{} — {}×{} @ {} Hz{}",
            name,
            if self.primary { " (principal)" } else { "" },
            self.width(),
            self.height(),
            self.refresh_hz,
            if self.hdr_active { " — HDR" } else { "" }
        )
    }

    /// Couverture approximative des gamuts standards à partir des primaires annoncées (aire CIE xy).
    pub fn gamut_coverage(&self) -> Option<(f32, f32, f32)> {
        let tri = [self.red_primary, self.green_primary, self.blue_primary];
        if area(tri) < 1e-4 {
            return None;
        }
        Some((
            overlap_ratio(tri, SRGB_PRIMARIES),
            overlap_ratio(tri, P3_PRIMARIES),
            overlap_ratio(tri, BT2020_PRIMARIES),
        ))
    }
}

pub const SRGB_PRIMARIES: [[f32; 2]; 3] = [[0.64, 0.33], [0.30, 0.60], [0.15, 0.06]];
pub const P3_PRIMARIES: [[f32; 2]; 3] = [[0.680, 0.320], [0.265, 0.690], [0.150, 0.060]];
pub const BT2020_PRIMARIES: [[f32; 2]; 3] = [[0.708, 0.292], [0.170, 0.797], [0.131, 0.046]];

fn area(t: [[f32; 2]; 3]) -> f32 {
    ((t[1][0] - t[0][0]) * (t[2][1] - t[0][1]) - (t[2][0] - t[0][0]) * (t[1][1] - t[0][1])).abs() * 0.5
}

/// Part du gamut de référence couverte par le triangle de l'écran (intersection de polygones convexes).
fn overlap_ratio(display: [[f32; 2]; 3], reference: [[f32; 2]; 3]) -> f32 {
    let mut poly: Vec<[f32; 2]> = reference.to_vec();
    let ccw = |t: [[f32; 2]; 3]| -> Vec<[f32; 2]> {
        let s = (t[1][0] - t[0][0]) * (t[2][1] - t[0][1]) - (t[2][0] - t[0][0]) * (t[1][1] - t[0][1]);
        if s < 0.0 { vec![t[0], t[2], t[1]] } else { t.to_vec() }
    };
    let clip = ccw(display);
    poly = ccw([poly[0], poly[1], poly[2]]);
    for i in 0..3 {
        let a = clip[i];
        let b = clip[(i + 1) % 3];
        let inside = |p: [f32; 2]| (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]) >= 0.0;
        let intersect = |p: [f32; 2], q: [f32; 2]| {
            let d1 = (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
            let d2 = (b[0] - a[0]) * (q[1] - a[1]) - (b[1] - a[1]) * (q[0] - a[0]);
            let t = d1 / (d1 - d2);
            [p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t]
        };
        let input = std::mem::take(&mut poly);
        for j in 0..input.len() {
            let cur = input[j];
            let prev = input[(j + input.len() - 1) % input.len()];
            match (inside(prev), inside(cur)) {
                (true, true) => poly.push(cur),
                (true, false) => poly.push(intersect(prev, cur)),
                (false, true) => {
                    poly.push(intersect(prev, cur));
                    poly.push(cur);
                }
                (false, false) => {}
            }
        }
        if poly.is_empty() {
            return 0.0;
        }
    }
    let mut a = 0.0;
    for i in 0..poly.len() {
        let p = poly[i];
        let q = poly[(i + 1) % poly.len()];
        a += p[0] * q[1] - q[0] * p[1];
    }
    (a.abs() * 0.5 / area(reference)).min(1.0)
}

fn wide_to_string(buf: &[u16]) -> String {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn color_space_name(cs: DXGI_COLOR_SPACE_TYPE) -> String {
    match cs {
        DXGI_COLOR_SPACE_RGB_FULL_G22_NONE_P709 => "sRGB (SDR, gamma 2.2, Rec.709)".into(),
        DXGI_COLOR_SPACE_RGB_FULL_G2084_NONE_P2020 => "HDR10 (PQ ST.2084, Rec.2020)".into(),
        DXGI_COLOR_SPACE_RGB_FULL_G10_NONE_P709 => "scRGB linéaire".into(),
        other => format!("Autre ({})", other.0),
    }
}

struct DisplayConfigData {
    friendly_name: String,
    supported: bool,
    enabled: bool,
    wide_enforced: bool,
    force_disabled: bool,
    bits: u32,
    sdr_white_nits: Option<f32>,
}

/// Interroge l'API DisplayConfig pour un nom GDI (\\.\DISPLAYn).
fn query_display_config(gdi_name: &str) -> Option<DisplayConfigData> {
    unsafe {
        let mut n_paths = 0u32;
        let mut n_modes = 0u32;
        if GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut n_paths, &mut n_modes) != ERROR_SUCCESS {
            return None;
        }
        let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); n_paths as usize];
        let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); n_modes as usize];
        if QueryDisplayConfig(
            QDC_ONLY_ACTIVE_PATHS,
            &mut n_paths,
            paths.as_mut_ptr(),
            &mut n_modes,
            modes.as_mut_ptr(),
            None,
        ) != ERROR_SUCCESS
        {
            return None;
        }
        paths.truncate(n_paths as usize);

        for path in &paths {
            let mut source = DISPLAYCONFIG_SOURCE_DEVICE_NAME::default();
            source.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME;
            source.header.size = size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32;
            source.header.adapterId = path.sourceInfo.adapterId;
            source.header.id = path.sourceInfo.id;
            if DisplayConfigGetDeviceInfo(&mut source.header) != 0 {
                continue;
            }
            if !wide_to_string(&source.viewGdiDeviceName).eq_ignore_ascii_case(gdi_name) {
                continue;
            }

            let mut target = DISPLAYCONFIG_TARGET_DEVICE_NAME::default();
            target.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
            target.header.size = size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32;
            target.header.adapterId = path.targetInfo.adapterId;
            target.header.id = path.targetInfo.id;
            let friendly_name = if DisplayConfigGetDeviceInfo(&mut target.header) == 0 {
                wide_to_string(&target.monitorFriendlyDeviceName)
            } else {
                String::new()
            };

            let mut color = DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO::default();
            color.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO;
            color.header.size = size_of::<DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO>() as u32;
            color.header.adapterId = path.targetInfo.adapterId;
            color.header.id = path.targetInfo.id;
            let (supported, enabled, wide_enforced, force_disabled, bits) =
                if DisplayConfigGetDeviceInfo(&mut color.header) == 0 {
                    let v = color.Anonymous.value;
                    (v & 1 != 0, v & 2 != 0, v & 4 != 0, v & 8 != 0, color.bitsPerColorChannel)
                } else {
                    (false, false, false, false, 0)
                };

            let mut white = DISPLAYCONFIG_SDR_WHITE_LEVEL::default();
            white.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_SDR_WHITE_LEVEL;
            white.header.size = size_of::<DISPLAYCONFIG_SDR_WHITE_LEVEL>() as u32;
            white.header.adapterId = path.targetInfo.adapterId;
            white.header.id = path.targetInfo.id;
            let sdr_white_nits = if DisplayConfigGetDeviceInfo(&mut white.header) == 0 {
                Some(white.SDRWhiteLevel as f32 / 1000.0 * 80.0)
            } else {
                None
            };

            return Some(DisplayConfigData {
                friendly_name,
                supported,
                enabled,
                wide_enforced,
                force_disabled,
                bits,
                sdr_white_nits,
            });
        }
        None
    }
}

fn icc_profile(gdi_name: &str) -> Option<String> {
    unsafe {
        let name = to_wide(gdi_name);
        let hdc = CreateDCW(w!("DISPLAY"), PCWSTR(name.as_ptr()), PCWSTR::null(), None);
        if hdc.is_invalid() {
            return None;
        }
        let mut buf = [0u16; 512];
        let mut len = buf.len() as u32;
        let ok = GetICMProfileW(hdc, &mut len, Some(PWSTR(buf.as_mut_ptr())));
        let _ = DeleteDC(hdc);
        if !ok.as_bool() {
            return None;
        }
        let path = wide_to_string(&buf);
        Some(path.rsplit('\\').next().unwrap_or(&path).to_string())
    }
}

fn refresh_rate(gdi_name: &str) -> u32 {
    unsafe {
        let name = to_wide(gdi_name);
        let mut mode = DEVMODEW { dmSize: size_of::<DEVMODEW>() as u16, ..Default::default() };
        if EnumDisplaySettingsW(PCWSTR(name.as_ptr()), ENUM_CURRENT_SETTINGS, &mut mode).as_bool() {
            mode.dmDisplayFrequency
        } else {
            0
        }
    }
}

fn is_primary(monitor: HMONITOR) -> bool {
    unsafe {
        let mut info = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
        GetMonitorInfoW(monitor, &mut info).as_bool() && info.dwFlags & MONITORINFOF_PRIMARY != 0
    }
}

pub fn enumerate() -> Vec<MonitorInfo> {
    let mut result = Vec::new();
    unsafe {
        let Ok(factory) = CreateDXGIFactory1::<IDXGIFactory1>() else {
            return result;
        };
        let mut a = 0;
        while let Ok(adapter) = factory.EnumAdapters1(a) {
            a += 1;
            let adapter_name = adapter.GetDesc1().map(|d| wide_to_string(&d.Description)).unwrap_or_default();
            let mut o = 0;
            while let Ok(output) = adapter.EnumOutputs(o) {
                o += 1;
                let Ok(output6) = output.cast::<IDXGIOutput6>() else { continue };
                let Ok(desc) = output6.GetDesc1() else { continue };
                if !desc.AttachedToDesktop.as_bool() {
                    continue;
                }
                let gdi_name = wide_to_string(&desc.DeviceName);
                let cfg = query_display_config(&gdi_name);
                let mut info = MonitorInfo {
                    adapter_name: adapter_name.clone(),
                    rect: desc.DesktopCoordinates,
                    refresh_hz: refresh_rate(&gdi_name),
                    primary: is_primary(desc.Monitor),
                    bits_per_color: desc.BitsPerColor,
                    hdr_active: desc.ColorSpace == DXGI_COLOR_SPACE_RGB_FULL_G2084_NONE_P2020,
                    color_space: color_space_name(desc.ColorSpace),
                    red_primary: desc.RedPrimary,
                    green_primary: desc.GreenPrimary,
                    blue_primary: desc.BluePrimary,
                    white_point: desc.WhitePoint,
                    min_luminance: desc.MinLuminance,
                    max_luminance: desc.MaxLuminance,
                    max_full_frame_luminance: desc.MaxFullFrameLuminance,
                    icc_profile: icc_profile(&gdi_name),
                    gdi_name,
                    ..Default::default()
                };
                if let Some(cfg) = cfg {
                    info.friendly_name = cfg.friendly_name;
                    info.advanced_color_supported = cfg.supported;
                    info.advanced_color_enabled = cfg.enabled;
                    info.wide_color_enforced = cfg.wide_enforced;
                    info.advanced_color_force_disabled = cfg.force_disabled;
                    if cfg.bits > 0 {
                        info.bits_per_color = cfg.bits;
                    }
                    if info.hdr_active {
                        info.sdr_white_nits = cfg.sdr_white_nits;
                    }
                }
                result.push(info);
            }
        }
    }
    result.sort_by_key(|m| (!m.primary, m.rect.left, m.rect.top));
    result
}
