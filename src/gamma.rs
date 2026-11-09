//! Correction logicielle via la table gamma (LUT) de la carte graphique.
//! Fonctionne en SDR ; Windows ignore la LUT GDI lorsque le HDR est actif.

use serde::{Deserialize, Serialize};
use windows::Win32::Graphics::Gdi::{CreateDCW, DeleteDC};
use windows::Win32::UI::ColorSystem::SetDeviceGammaRamp;
use windows::core::{PCWSTR, w};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GammaSettings {
    /// Gamma mesuré par canal (R, G, B) avec la mire gamma, LUT réinitialisée.
    pub measured_gamma: [f32; 3],
    pub target_gamma: f32,
    /// Gain du blanc par canal (balance des blancs dans les hautes lumières).
    pub gain: [f32; 3],
    /// Décalage du noir par canal (balance dans les basses lumières).
    pub black: [f32; 3],
}

impl Default for GammaSettings {
    fn default() -> Self {
        Self { measured_gamma: [2.2; 3], target_gamma: 2.2, gain: [1.0; 3], black: [0.0; 3] }
    }
}

impl GammaSettings {
    pub fn curve(&self, channel: usize, x: f32) -> f32 {
        let exponent = self.target_gamma / self.measured_gamma[channel].max(0.1);
        let y = x.powf(exponent) * self.gain[channel] + self.black[channel] * (1.0 - x);
        y.clamp(0.0, 1.0)
    }

    fn ramp(&self) -> [[u16; 256]; 3] {
        let mut ramp = [[0u16; 256]; 3];
        for (c, channel) in ramp.iter_mut().enumerate() {
            for (i, v) in channel.iter_mut().enumerate() {
                *v = (self.curve(c, i as f32 / 255.0) * 65535.0).round() as u16;
            }
        }
        ramp
    }
}

fn set_ramp(gdi_name: &str, ramp: &[[u16; 256]; 3]) -> Result<(), String> {
    let name: Vec<u16> = gdi_name.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let hdc = CreateDCW(w!("DISPLAY"), PCWSTR(name.as_ptr()), PCWSTR::null(), None);
        if hdc.is_invalid() {
            return Err(format!("Impossible d'ouvrir le contexte de {gdi_name}"));
        }
        let ok = SetDeviceGammaRamp(hdc, ramp.as_ptr() as *const _);
        let _ = DeleteDC(hdc);
        if ok.as_bool() {
            Ok(())
        } else {
            Err("Windows a refusé la table gamma. Correction trop forte : Windows limite l'écart \
                 par rapport à la courbe neutre (voir l'onglet Aide pour lever la limite)."
                .into())
        }
    }
}

pub fn apply(gdi_name: &str, settings: &GammaSettings) -> Result<(), String> {
    set_ramp(gdi_name, &settings.ramp())
}

pub fn reset(gdi_name: &str) -> Result<(), String> {
    set_ramp(gdi_name, &GammaSettings::default().ramp())
}
