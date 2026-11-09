//! Fenêtre plein écran (sans bordure) affichant les mires via une swap chain DXGI :
//! B8G8R8A8 en SDR (codes exacts), R16G16B16A16_FLOAT scRGB en HDR (luminances absolues).

use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::*;
use windows::Win32::Graphics::Gdi::{BLACK_BRUSH, GetStockObject, HBRUSH};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetThreadDpiAwarenessContext};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{Interface, Result, w};

use crate::display::MonitorInfo;
use crate::patterns::{self, Canvas, Pattern, PATTERN_COUNT};

pub struct Shared {
    pub pattern: Pattern,
    pub params: [usize; PATTERN_COUNT],
    pub overlay: bool,
    pub open: bool,
    pub close_request: bool,
    pub hdr: bool,
    pub revision: u64,
    pub error: Option<String>,
}

impl Default for Shared {
    fn default() -> Self {
        let mut params = [0; PATTERN_COUNT];
        for p in Pattern::SDR.iter().chain(Pattern::HDR.iter()) {
            params[p.index()] = p.default_param();
        }
        Self {
            pattern: Pattern::BlackLevel,
            params,
            overlay: true,
            open: false,
            close_request: false,
            hdr: false,
            revision: 0,
            error: None,
        }
    }
}

impl Shared {
    pub fn param(&self) -> usize {
        self.params[self.pattern.index()]
    }

    pub fn set_pattern(&mut self, p: Pattern) {
        self.pattern = p;
        self.revision += 1;
    }

    pub fn step_pattern(&mut self, delta: i32) {
        let list = Pattern::available(self.hdr);
        let i = list.iter().position(|&p| p == self.pattern).unwrap_or(0) as i32;
        let n = list.len() as i32;
        self.set_pattern(list[((i + delta) % n + n) as usize % n as usize]);
    }

    pub fn step_param(&mut self, delta: i32) {
        let n = self.pattern.param_count() as i32;
        let cur = self.param() as i32;
        self.params[self.pattern.index()] = (cur + delta).clamp(0, n - 1) as usize;
        self.revision += 1;
    }
}

pub type SharedState = Arc<Mutex<Shared>>;

thread_local! {
    static STATE: RefCell<Option<SharedState>> = const { RefCell::new(None) };
}

fn with_state(f: impl FnOnce(&mut Shared)) {
    STATE.with(|s| {
        if let Some(state) = s.borrow().as_ref() {
            if let Ok(mut st) = state.lock() {
                f(&mut st);
            }
        }
    });
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_KEYDOWN => {
                let key = VIRTUAL_KEY(wparam.0 as u16);
                with_state(|st| match key {
                    VK_ESCAPE => st.close_request = true,
                    VK_RIGHT | VK_SPACE | VK_NEXT => st.step_pattern(1),
                    VK_LEFT | VK_PRIOR => st.step_pattern(-1),
                    VK_UP => st.step_param(1),
                    VK_DOWN => st.step_param(-1),
                    VK_H => {
                        st.overlay = !st.overlay;
                        st.revision += 1;
                    }
                    _ => {}
                });
                LRESULT(0)
            }
            WM_LBUTTONUP => {
                with_state(|st| st.step_pattern(1));
                LRESULT(0)
            }
            WM_RBUTTONUP => {
                with_state(|st| st.step_pattern(-1));
                LRESULT(0)
            }
            WM_MOUSEWHEEL => {
                let delta = (wparam.0 >> 16) as u16 as i16;
                with_state(|st| st.step_param(if delta > 0 { 1 } else { -1 }));
                LRESULT(0)
            }
            WM_SETCURSOR if (lparam.0 & 0xFFFF) as u32 == HTCLIENT => {
                SetCursor(None);
                LRESULT(1)
            }
            WM_CLOSE => {
                with_state(|st| st.close_request = true);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

struct Renderer {
    swap_chain: IDXGISwapChain1,
    context: ID3D11DeviceContext,
}

impl Renderer {
    fn new(hwnd: HWND, w: u32, h: u32, hdr: bool) -> Result<Self> {
        unsafe {
            let mut device = None;
            let mut context = None;
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                Some(&[D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_10_0]),
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            )?;
            let device: ID3D11Device = device.unwrap();
            let context = context.unwrap();

            let factory: IDXGIFactory2 = CreateDXGIFactory2(DXGI_CREATE_FACTORY_FLAGS(0))?;
            let desc = DXGI_SWAP_CHAIN_DESC1 {
                Width: w,
                Height: h,
                Format: if hdr { DXGI_FORMAT_R16G16B16A16_FLOAT } else { DXGI_FORMAT_B8G8R8A8_UNORM },
                SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                BufferCount: 2,
                Scaling: DXGI_SCALING_NONE,
                SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
                AlphaMode: DXGI_ALPHA_MODE_IGNORE,
                ..Default::default()
            };
            let swap_chain = factory.CreateSwapChainForHwnd(&device, hwnd, &desc, None, None)?;
            let _ = factory.MakeWindowAssociation(hwnd, DXGI_MWA_NO_ALT_ENTER);
            if hdr {
                let sc3: IDXGISwapChain3 = swap_chain.cast()?;
                sc3.SetColorSpace1(DXGI_COLOR_SPACE_RGB_FULL_G10_NONE_P709)?;
            }
            Ok(Self { swap_chain, context })
        }
    }

    fn present(&self, canvas: &Canvas) -> Result<()> {
        unsafe {
            let back: ID3D11Texture2D = self.swap_chain.GetBuffer(0)?;
            self.context.UpdateSubresource(&back, 0, None, canvas.data_ptr(), canvas.row_pitch(), 0);
            self.swap_chain.Present(1, DXGI_PRESENT(0)).ok()
        }
    }
}

fn run(state: SharedState, monitor: MonitorInfo) -> Result<()> {
    unsafe {
        SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let instance = GetModuleHandleW(None)?;
        let class = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(wndproc),
            hInstance: instance.into(),
            hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
            lpszClassName: w!("ScreenCalibrationPatterns"),
            ..Default::default()
        };
        RegisterClassExW(&class);

        let (w, h) = (monitor.width(), monitor.height());
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("ScreenCalibrationPatterns"),
            w!("Mires de calibration"),
            WS_POPUP | WS_VISIBLE,
            monitor.rect.left,
            monitor.rect.top,
            w,
            h,
            None,
            None,
            Some(instance.into()),
            None,
        )?;
        let _ = SetForegroundWindow(hwnd);

        let hdr = monitor.hdr_active;
        let sdr_white = monitor.sdr_white_nits.unwrap_or(80.0);
        let renderer = Renderer::new(hwnd, w as u32, h as u32, hdr)?;
        let mut canvas = Canvas::new(w, h, hdr, sdr_white);
        let mut last_revision = u64::MAX;

        loop {
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            let snapshot = {
                let st = state.lock().unwrap();
                if st.close_request {
                    break;
                }
                (st.revision != last_revision).then(|| (st.revision, st.pattern, st.param(), st.overlay))
            };

            if let Some((revision, pattern, param, overlay)) = snapshot {
                last_revision = revision;
                patterns::render(&mut canvas, pattern, param);
                if overlay {
                    let scale = ((h as f32 / 1080.0 * 2.0).round() as i32).max(1);
                    let mut lines = vec![pattern.title().to_string()];
                    if let Some(label) = pattern.param_label(param) {
                        lines.push(label);
                    }
                    lines.push("<- -> / clic : mire   Haut/Bas / molette : réglage".into());
                    lines.push("H : masquer ce texte   Échap : quitter".into());
                    canvas.overlay(&lines, scale);
                }
                renderer.present(&canvas)?;
            }
            std::thread::sleep(Duration::from_millis(15));
        }
        let _ = DestroyWindow(hwnd);
        Ok(())
    }
}

/// Ouvre la fenêtre de mires sur l'écran choisi, dans un thread dédié.
pub fn open(state: SharedState, monitor: MonitorInfo) {
    {
        let mut st = state.lock().unwrap();
        if st.open {
            return;
        }
        st.open = true;
        st.close_request = false;
        st.error = None;
        st.hdr = monitor.hdr_active;
        if !st.hdr && st.pattern.is_hdr() {
            st.pattern = Pattern::BlackLevel;
        }
        st.revision += 1;
    }
    std::thread::spawn(move || {
        STATE.with(|s| *s.borrow_mut() = Some(state.clone()));
        let result = run(state.clone(), monitor);
        let mut st = state.lock().unwrap();
        st.open = false;
        st.close_request = false;
        if let Err(e) = result {
            st.error = Some(format!("Erreur d'affichage des mires : {e}"));
        }
    });
}
