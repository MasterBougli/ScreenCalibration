//! Mires de test rendues sur CPU, au pixel physique près.
//! Les couleurs sont exprimées soit en code SDR (0..1, encodé sRGB), soit en nits absolus
//! (linéaire, primaires Rec.709 ; les valeurs négatives représentent des couleurs hors sRGB en scRGB).

use font8x8::UnicodeFonts;
use half::f16;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pattern {
    BlackLevel,
    WhiteClipping,
    Gamma,
    GrayRamp,
    Gradients,
    ColorBars,
    Saturation,
    Sharpness,
    Uniformity,
    DeadPixels,
    HdrPeak10,
    HdrPeakFull,
    HdrBlack,
    HdrSteps,
    HdrSdrWhite,
    HdrGamut,
    HdrGradient,
}

pub const PATTERN_COUNT: usize = 17;

const UNIFORMITY_LEVELS: [u8; 7] = [0, 5, 10, 25, 50, 75, 100];
const DEAD_PIXEL_COLORS: [(&str, [f32; 3]); 8] = [
    ("Noir", [0.0, 0.0, 0.0]),
    ("Blanc", [1.0, 1.0, 1.0]),
    ("Rouge", [1.0, 0.0, 0.0]),
    ("Vert", [0.0, 1.0, 0.0]),
    ("Bleu", [0.0, 0.0, 1.0]),
    ("Cyan", [0.0, 1.0, 1.0]),
    ("Magenta", [1.0, 0.0, 1.0]),
    ("Jaune", [1.0, 1.0, 0.0]),
];
const GAMMA_CHANNELS: [&str; 4] = ["Gris", "Rouge", "Vert", "Bleu"];
const RAMP_STEPS: [usize; 4] = [11, 16, 32, 64];
pub const PEAK_NITS: [f32; 24] = [
    100.0, 200.0, 300.0, 400.0, 500.0, 600.0, 700.0, 800.0, 900.0, 1000.0, 1100.0, 1200.0, 1300.0, 1400.0,
    1500.0, 1600.0, 1800.0, 2000.0, 2500.0, 3000.0, 4000.0, 5000.0, 6000.0, 10000.0,
];
pub const BLACK_NITS: [f32; 14] =
    [0.0, 0.0025, 0.005, 0.0075, 0.01, 0.02, 0.03, 0.05, 0.075, 0.1, 0.15, 0.2, 0.3, 0.5];
const STEP_NITS: [f32; 26] = [
    0.05, 0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 80.0, 100.0, 150.0, 203.0, 300.0, 400.0, 500.0, 600.0, 800.0,
    1000.0, 1200.0, 1500.0, 2000.0, 3000.0, 4000.0, 6000.0, 10000.0,
];
const GAMUT_WHITE: [f32; 4] = [100.0, 203.0, 400.0, 1000.0];
const GRADIENT_MAX: [f32; 5] = [100.0, 400.0, 1000.0, 4000.0, 10000.0];

impl Pattern {
    pub const SDR: [Pattern; 10] = [
        Pattern::BlackLevel,
        Pattern::WhiteClipping,
        Pattern::Gamma,
        Pattern::GrayRamp,
        Pattern::Gradients,
        Pattern::ColorBars,
        Pattern::Saturation,
        Pattern::Sharpness,
        Pattern::Uniformity,
        Pattern::DeadPixels,
    ];
    pub const HDR: [Pattern; 7] = [
        Pattern::HdrPeak10,
        Pattern::HdrPeakFull,
        Pattern::HdrBlack,
        Pattern::HdrSteps,
        Pattern::HdrSdrWhite,
        Pattern::HdrGamut,
        Pattern::HdrGradient,
    ];

    pub fn available(hdr: bool) -> Vec<Pattern> {
        let mut v = Self::SDR.to_vec();
        if hdr {
            v.extend_from_slice(&Self::HDR);
        }
        v
    }

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn is_hdr(self) -> bool {
        Self::HDR.contains(&self)
    }

    pub fn title(self) -> &'static str {
        match self {
            Pattern::BlackLevel => "Niveau de noir (luminosité)",
            Pattern::WhiteClipping => "Écrêtage du blanc (contraste)",
            Pattern::Gamma => "Gamma",
            Pattern::GrayRamp => "Échelle de gris (balance des blancs)",
            Pattern::Gradients => "Dégradés (banding)",
            Pattern::ColorBars => "Barres de couleurs",
            Pattern::Saturation => "Saturation",
            Pattern::Sharpness => "Netteté",
            Pattern::Uniformity => "Uniformité / fuites de lumière",
            Pattern::DeadPixels => "Pixels morts / bloqués",
            Pattern::HdrPeak10 => "HDR : luminance max (fenêtre 10 %)",
            Pattern::HdrPeakFull => "HDR : luminance max (plein écran)",
            Pattern::HdrBlack => "HDR : luminance minimale (noir)",
            Pattern::HdrSteps => "HDR : paliers de luminance (tone mapping)",
            Pattern::HdrSdrWhite => "HDR : blanc SDR vs référence HDR",
            Pattern::HdrGamut => "HDR : gamut (Rec.709 / DCI-P3 / Rec.2020)",
            Pattern::HdrGradient => "HDR : dégradés PQ",
        }
    }

    pub fn instructions(self) -> &'static str {
        match self {
            Pattern::BlackLevel => {
                "Fond noir pur avec des carrés de niveau 1 à 30 (sur 255).\n\
                 Réglez la LUMINOSITÉ (ou « niveau de noir ») de l'écran : augmentez-la jusqu'à distinguer \
                 les carrés les plus sombres (idéalement dès 1–3), sans que le fond noir devienne gris.\n\
                 Si aucun carré bas n'apparaît même au maximum, vérifiez la plage RGB (Complète 0-255 vs \
                 Limitée 16-235) dans le panneau du pilote graphique et dans le menu de l'écran : elles doivent concorder."
            }
            Pattern::WhiteClipping => {
                "Fond blanc pur avec des carrés de niveau 226 à 254.\n\
                 Réglez le CONTRASTE : le plus haut possible tant que tous les carrés restent distincts du fond \
                 (au moins jusqu'à 250–252). Si les carrés clairs se fondent dans le blanc, baissez le contraste.\n\
                 Vérifiez aussi qu'aucune teinte colorée n'apparaît sur les carrés clairs."
            }
            Pattern::Gamma => {
                "Chaque case contient des lignes noires/blanches de 1 pixel (luminance moyenne = 50 %) et un carré \
                 uni au centre calculé pour un gamma donné.\n\
                 Reculez de 2–3 m ou plissez les yeux : la case où le carré central se fond dans les lignes indique \
                 le gamma réel de l'écran. Cible usuelle : 2.2 (bureautique/web/jeux), 2.4 (pièce sombre/vidéo).\n\
                 Ajustez l'option gamma de l'écran, ou saisissez la valeur trouvée dans « Correction gamma (SDR) ». \
                 ↑/↓ : canal Gris / Rouge / Vert / Bleu pour mesurer chaque canal séparément.\n\
                 Mesurez avec la correction réinitialisée. La mise à l'échelle Windows n'a pas d'effet : la mire est \
                 rendue au pixel physique."
            }
            Pattern::GrayRamp => {
                "Paliers de gris du noir au blanc (haut) et dégradé continu (bas).\n\
                 Chaque palier doit être neutre (aucune dominante rose, verte ou bleue) et distinct de ses voisins.\n\
                 Dominante dans les clairs : ajustez les gains RVB / température de couleur de l'écran (6500 K / sRGB). \
                 Dominante dans les sombres : ajustez les « offsets » RVB si l'écran le permet, ou le noir RVB ici.\n\
                 ↑/↓ : nombre de paliers."
            }
            Pattern::Gradients => {
                "Dégradés continus gris, rouge, vert, bleu, puis un dégradé très sombre étiré (0→32).\n\
                 Des bandes visibles (banding) indiquent une perte de précision : traitement de l'écran \
                 (contraste dynamique, modes « vivid »), une correction gamma trop forte ou un affichage en 6 bits.\n\
                 Désactivez les modes d'amélioration d'image et préférez la sortie 8/10 bits dans le pilote."
            }
            Pattern::ColorBars => {
                "Barres de couleurs à 75 % (haut) et 100 % (bas).\n\
                 Les couleurs doivent être franches, sans dégradé ni écrêtage ; le jaune ne doit pas tirer vers le vert, \
                 le blanc doit rester neutre. Si les couleurs sont criardes, vérifiez que le mode « sRGB » de l'écran est \
                 actif pour le contenu SDR (sur un écran à large gamut sans gestion des couleurs)."
            }
            Pattern::Saturation => {
                "Rampes de saturation de 10 % à 100 % pour chaque primaire et secondaire.\n\
                 Toutes les cases doivent être différentes. Si les dernières cases se confondent, la saturation de \
                 l'écran (ou la vibrance du pilote) est trop élevée : remettez-la à sa valeur neutre."
            }
            Pattern::Sharpness => {
                "Lignes de 1 pixel, damiers et texte.\n\
                 Réglez la NETTETÉ de l'écran sur la valeur la plus basse ne rendant pas l'image floue : aucun halo \
                 clair/sombre ne doit apparaître le long des lignes ni autour du texte. Les damiers doivent rester \
                 uniformes (sans moiré) : sinon la résolution n'est pas la résolution native ou une mise à l'échelle est active."
            }
            Pattern::Uniformity => {
                "Plein écran uni (↑/↓ : niveau).\n\
                 Cherchez les zones plus claires/sombres ou teintées, et en 0 % (dans le noir complet) les fuites \
                 de lumière (backlight bleed) ou l'IPS glow dans les coins. C'est un diagnostic : ces défauts ne se \
                 corrigent pas par réglage."
            }
            Pattern::DeadPixels => {
                "Plein écran d'une couleur unie (↑/↓ : couleur).\n\
                 Inspectez de près : un pixel noir sur fond blanc = pixel mort ; un pixel coloré allumé sur fond \
                 noir = pixel bloqué."
            }
            Pattern::HdrPeak10 => {
                "Fond noir, carré extérieur à 10 000 nits (écrêté à la capacité réelle de l'écran) et carré central \
                 à la valeur testée.\n\
                 ↑/↓ : augmentez la valeur jusqu'à ce que le carré central disparaisse. La dernière valeur où il était \
                 encore visible = luminance de crête réelle (fenêtre 10 %). Reportez-la dans les jeux (« luminance max ») \
                 et dans l'app Windows HDR Calibration."
            }
            Pattern::HdrPeakFull => {
                "Écran entier à 10 000 nits sauf le carré central à la valeur testée.\n\
                 ↑/↓ : augmentez jusqu'à ce que le carré disparaisse = luminance plein écran (souvent bien plus basse \
                 sur OLED à cause de l'ABL). Valeur utile pour le paramètre « luminance plein écran » des jeux."
            }
            Pattern::HdrBlack => {
                "Fond à 0 nit et carré central à la valeur testée.\n\
                 ↑/↓ : partez de 0 et augmentez jusqu'à ce que le carré devienne visible = luminance minimale perceptible. \
                 Si même 0.5 nit est invisible, le noir est écrasé (vérifiez le « Black Equalizer », le niveau de noir HDMI, \
                 ou la gestion locale de la luminance)."
            }
            Pattern::HdrSteps => {
                "Paliers de 0,05 à 10 000 nits.\n\
                 Les paliers doivent rester distincts jusqu'à la luminance de crête de l'écran. Au-delà, ils deviennent \
                 identiques (écrêtage). S'ils se resserrent progressivement, l'écran applique un tone mapping (ex. HGIG \
                 désactivé). Pour un rendu fidèle, activez HGIG / désactivez le « dynamic tone mapping » de l'écran."
            }
            Pattern::HdrSdrWhite => {
                "Gauche : blanc SDR tel que Windows l'affiche (curseur « Luminosité du contenu SDR »). Centre : blanc de \
                 référence HDR (203 nits, BT.2408). Droite : 100 nits.\n\
                 Ajustez le curseur SDR dans Paramètres > Affichage > HDR pour un blanc SDR confortable, typiquement \
                 proche de la référence 203 nits en pièce éclairée, ~100–160 nits en pièce sombre."
            }
            Pattern::HdrGamut => {
                "Primaires rouge, vert, bleu dans trois espaces : Rec.709 (sRGB), DCI-P3 et Rec.2020.\n\
                 Si l'écran couvre le DCI-P3, la colonne P3 est nettement plus saturée que Rec.709. Si P3 et Rec.2020 \
                 sont identiques, l'écran est limité au P3 (normal pour presque tous les écrans). \
                 ↑/↓ : luminance du blanc de référence."
            }
            Pattern::HdrGradient => {
                "Dégradés en espacement perceptuel PQ de 0 jusqu'à la valeur choisie (↑/↓).\n\
                 Aucune bande ne doit apparaître. Du banding indique un signal 8 bits sans dithering ou un traitement \
                 de l'écran : préférez 10 bits (ou 8 bits + dithering) dans le pilote graphique."
            }
        }
    }

    pub fn param_count(self) -> usize {
        match self {
            Pattern::Gamma => GAMMA_CHANNELS.len(),
            Pattern::GrayRamp => RAMP_STEPS.len(),
            Pattern::Uniformity => UNIFORMITY_LEVELS.len(),
            Pattern::DeadPixels => DEAD_PIXEL_COLORS.len(),
            Pattern::HdrPeak10 | Pattern::HdrPeakFull => PEAK_NITS.len(),
            Pattern::HdrBlack => BLACK_NITS.len(),
            Pattern::HdrGamut => GAMUT_WHITE.len(),
            Pattern::HdrGradient => GRADIENT_MAX.len(),
            _ => 1,
        }
    }

    pub fn default_param(self) -> usize {
        match self {
            Pattern::GrayRamp => 1,
            Pattern::Uniformity => 4,
            Pattern::HdrPeak10 | Pattern::HdrPeakFull => 3,
            Pattern::HdrGamut => 1,
            Pattern::HdrGradient => 2,
            _ => 0,
        }
    }

    pub fn param_label(self, p: usize) -> Option<String> {
        let p = p.min(self.param_count() - 1);
        Some(match self {
            Pattern::Gamma => format!("Canal : {}", GAMMA_CHANNELS[p]),
            Pattern::GrayRamp => format!("{} paliers", RAMP_STEPS[p]),
            Pattern::Uniformity => format!("Niveau : {} %", UNIFORMITY_LEVELS[p]),
            Pattern::DeadPixels => format!("Couleur : {}", DEAD_PIXEL_COLORS[p].0),
            Pattern::HdrPeak10 | Pattern::HdrPeakFull => format!("Carré central : {} nits", PEAK_NITS[p]),
            Pattern::HdrBlack => format!("Carré central : {} nit", BLACK_NITS[p]),
            Pattern::HdrGamut => format!("Blanc de référence : {} nits", GAMUT_WHITE[p]),
            Pattern::HdrGradient => format!("Maximum : {} nits", GRADIENT_MAX[p]),
            _ => return None,
        })
    }
}

#[derive(Clone, Copy)]
pub enum Px {
    /// Code SDR encodé (0..1 par canal), ce qu'une application SDR enverrait.
    Code([f32; 3]),
    /// Luminance absolue en nits (linéaire, primaires Rec.709).
    Nits([f32; 3]),
}

fn gray(v: f32) -> Px {
    Px::Code([v, v, v])
}

fn code8(v: u8) -> Px {
    gray(v as f32 / 255.0)
}

fn nits(v: f32) -> Px {
    Px::Nits([v, v, v])
}

pub fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

pub fn linear_to_srgb(l: f32) -> f32 {
    if l <= 0.0031308 { l * 12.92 } else { 1.055 * l.powf(1.0 / 2.4) - 0.055 }
}

const PQ_M1: f32 = 0.159_301_76;
const PQ_M2: f32 = 78.84375;
const PQ_C1: f32 = 0.835_937_5;
const PQ_C2: f32 = 18.851_563;
const PQ_C3: f32 = 18.6875;

fn pq_encode(nits: f32) -> f32 {
    let y = (nits / 10000.0).clamp(0.0, 1.0).powf(PQ_M1);
    ((PQ_C1 + PQ_C2 * y) / (1.0 + PQ_C3 * y)).powf(PQ_M2)
}

fn pq_decode(e: f32) -> f32 {
    let p = e.clamp(0.0, 1.0).powf(1.0 / PQ_M2);
    ((p - PQ_C1).max(0.0) / (PQ_C2 - PQ_C3 * p)).powf(1.0 / PQ_M1) * 10000.0
}

pub enum Buffer {
    /// B8G8R8A8_UNORM
    Sdr(Vec<u32>),
    /// R16G16B16A16_FLOAT scRGB (1.0 = 80 nits)
    Hdr(Vec<u64>),
}

pub struct Canvas {
    pub w: i32,
    pub h: i32,
    pub buf: Buffer,
    /// Luminance du blanc SDR en mode HDR (curseur Windows).
    pub sdr_white: f32,
}

impl Canvas {
    pub fn new(w: i32, h: i32, hdr: bool, sdr_white: f32) -> Self {
        let n = (w.max(1) * h.max(1)) as usize;
        let buf = if hdr { Buffer::Hdr(vec![0; n]) } else { Buffer::Sdr(vec![0; n]) };
        Self { w, h, buf, sdr_white }
    }

    pub fn row_pitch(&self) -> u32 {
        match self.buf {
            Buffer::Sdr(_) => self.w as u32 * 4,
            Buffer::Hdr(_) => self.w as u32 * 8,
        }
    }

    pub fn data_ptr(&self) -> *const std::ffi::c_void {
        match &self.buf {
            Buffer::Sdr(v) => v.as_ptr() as *const _,
            Buffer::Hdr(v) => v.as_ptr() as *const _,
        }
    }

    fn pack_sdr(px: Px) -> u32 {
        let c = match px {
            Px::Code(c) => c,
            Px::Nits(n) => n.map(|v| linear_to_srgb((v / 100.0).clamp(0.0, 1.0))),
        };
        let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
        0xFF00_0000 | (q(c[0]) << 16) | (q(c[1]) << 8) | q(c[2])
    }

    fn pack_hdr(&self, px: Px) -> u64 {
        let lin = match px {
            Px::Code(c) => c.map(|v| srgb_to_linear(v.clamp(0.0, 1.0)) * self.sdr_white / 80.0),
            Px::Nits(n) => n.map(|v| v / 80.0),
        };
        let h = |v: f32| f16::from_f32(v).to_bits() as u64;
        h(lin[0]) | (h(lin[1]) << 16) | (h(lin[2]) << 32) | (h(1.0) << 48)
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, px: Px) {
        let x0 = x.clamp(0, self.w);
        let y0 = y.clamp(0, self.h);
        let x1 = (x + w).clamp(0, self.w);
        let y1 = (y + h).clamp(0, self.h);
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        let stride = self.w as usize;
        match &mut self.buf {
            Buffer::Sdr(v) => {
                let p = Self::pack_sdr(px);
                for row in y0..y1 {
                    let start = row as usize * stride;
                    v[start + x0 as usize..start + x1 as usize].fill(p);
                }
            }
            Buffer::Hdr(_) => {
                let p = self.pack_hdr(px);
                let Buffer::Hdr(v) = &mut self.buf else { unreachable!() };
                for row in y0..y1 {
                    let start = row as usize * stride;
                    v[start + x0 as usize..start + x1 as usize].fill(p);
                }
            }
        }
    }

    pub fn fill(&mut self, px: Px) {
        self.fill_rect(0, 0, self.w, self.h, px);
    }

    pub fn text_width(text: &str, scale: i32) -> i32 {
        text.chars().count() as i32 * 8 * scale
    }

    pub fn text(&mut self, x: i32, y: i32, scale: i32, text: &str, px: Px) {
        let mut cx = x;
        for ch in text.chars() {
            let glyph = font8x8::BASIC_FONTS.get(ch).or_else(|| font8x8::LATIN_FONTS.get(ch)).unwrap_or([0; 8]);
            for (row, bits) in glyph.iter().enumerate() {
                for col in 0..8 {
                    if bits & (1 << col) != 0 {
                        self.fill_rect(cx + col * scale, y + row as i32 * scale, scale, scale, px);
                    }
                }
            }
            cx += 8 * scale;
        }
    }

    pub fn text_centered(&mut self, cx: i32, y: i32, scale: i32, text: &str, px: Px) {
        self.text(cx - Self::text_width(text, scale) / 2, y, scale, text, px);
    }

    /// Boîte d'information lisible, en bas à gauche.
    pub fn overlay(&mut self, lines: &[String], scale: i32) {
        let pad = 6 * scale;
        let line_h = 11 * scale;
        let width = lines.iter().map(|l| Self::text_width(l, scale)).max().unwrap_or(0) + pad * 2;
        let height = lines.len() as i32 * line_h + pad * 2 - 3 * scale;
        let x = 16 * scale;
        let y = self.h - height - 16 * scale;
        self.fill_rect(x, y, width, height, gray(0.12));
        for (i, line) in lines.iter().enumerate() {
            let color = if i == 0 { gray(0.9) } else { gray(0.7) };
            self.text(x + pad, y + pad + i as i32 * line_h, scale, line, color);
        }
    }
}

pub fn render(c: &mut Canvas, pattern: Pattern, param: usize) {
    let (w, h) = (c.w, c.h);
    let u = (h as f32 / 1080.0).max(0.5);
    let s = ((2.0 * u).round() as i32).max(1);
    let p = param.min(pattern.param_count() - 1);

    match pattern {
        Pattern::BlackLevel => {
            c.fill(code8(0));
            let rows: [Vec<u8>; 2] = [(1..=10).collect(), (12..=30).step_by(2).collect()];
            draw_level_rows(c, &rows, code8(64), s);
        }
        Pattern::WhiteClipping => {
            c.fill(code8(255));
            let rows: [Vec<u8>; 2] = [(245..=254).collect(), (226..=244).step_by(2).collect()];
            draw_level_rows(c, &rows, code8(150), s);
        }
        Pattern::Gamma => {
            c.fill(code8(0));
            let gammas = [1.8f32, 1.9, 2.0, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6];
            let n = gammas.len() as i32;
            let size = (w / (n + 1)).min(h / 3);
            let gap = (w - size * n) / (n + 1);
            let y = (h - size) / 2;
            let mask = match p {
                1 => [1.0, 0.0, 0.0],
                2 => [0.0, 1.0, 0.0],
                3 => [0.0, 0.0, 1.0],
                _ => [1.0, 1.0, 1.0],
            };
            let on = Px::Code(mask);
            for (i, g) in gammas.iter().enumerate() {
                let x = gap + i as i32 * (size + gap);
                for row in (0..size).step_by(2) {
                    c.fill_rect(x, y + row, size, 1, on);
                }
                let v = 0.5f32.powf(1.0 / g);
                let inner = size / 2;
                c.fill_rect(x + inner / 2, y + inner / 2, inner, inner, Px::Code(mask.map(|m| m * v)));
                c.text_centered(x + size / 2, y + size + 12 * s, s, &format!("{g:.1}"), code8(170));
            }
            c.text_centered(w / 2, y - 24 * s, s, GAMMA_CHANNELS[p], code8(170));
        }
        Pattern::GrayRamp => {
            c.fill(code8(0));
            let n = RAMP_STEPS[p] as i32;
            let top = h / 2;
            for i in 0..n {
                let x0 = i * w / n;
                let x1 = (i + 1) * w / n;
                let v = i as f32 / (n - 1) as f32;
                c.fill_rect(x0, 0, x1 - x0, top, gray(v));
                let label = format!("{}", (v * 100.0).round() as i32);
                if Canvas::text_width(&label, s) + 4 * s < x1 - x0 {
                    let col = if v > 0.5 { code8(0) } else { code8(200) };
                    c.text_centered((x0 + x1) / 2, top - 16 * s, s, &label, col);
                }
            }
            for x in 0..w {
                c.fill_rect(x, top, 1, h - top, gray(x as f32 / (w - 1) as f32));
            }
        }
        Pattern::Gradients => {
            c.fill(code8(0));
            let bands: [([f32; 3], f32); 5] = [
                ([1.0, 1.0, 1.0], 1.0),
                ([1.0, 0.0, 0.0], 1.0),
                ([0.0, 1.0, 0.0], 1.0),
                ([0.0, 0.0, 1.0], 1.0),
                ([1.0, 1.0, 1.0], 32.0 / 255.0),
            ];
            let bh = h / bands.len() as i32;
            for (b, (col, max)) in bands.iter().enumerate() {
                for x in 0..w {
                    let v = x as f32 / (w - 1) as f32 * max;
                    let v = (v * 255.0).round() / 255.0;
                    c.fill_rect(x, b as i32 * bh, 1, bh, Px::Code(col.map(|m| m * v)));
                }
            }
        }
        Pattern::ColorBars => {
            let cols: [[f32; 3]; 8] = [
                [1.0, 1.0, 1.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 1.0],
                [0.0, 1.0, 0.0],
                [1.0, 0.0, 1.0],
                [1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 0.0],
            ];
            let split = h * 2 / 3;
            for (i, col) in cols.iter().enumerate() {
                let x0 = i as i32 * w / 8;
                let x1 = (i as i32 + 1) * w / 8;
                c.fill_rect(x0, 0, x1 - x0, split, Px::Code(col.map(|m| m * 0.75)));
                c.fill_rect(x0, split, x1 - x0, h - split, Px::Code(*col));
            }
        }
        Pattern::Saturation => {
            c.fill(code8(0));
            let cols: [[f32; 3]; 6] = [
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 1.0, 1.0],
                [0.0, 0.0, 1.0],
                [1.0, 0.0, 1.0],
            ];
            let n = 10;
            let margin = 40 * s;
            let cw = (w - 2 * margin) / n;
            let ch = (h - 2 * margin) / cols.len() as i32;
            for (r, col) in cols.iter().enumerate() {
                for i in 0..n {
                    let sat = (i + 1) as f32 / n as f32;
                    let px = Px::Code(col.map(|m| 1.0 - sat + sat * m));
                    c.fill_rect(margin + i * cw + 2, margin + r as i32 * ch + 2, cw - 4, ch - 4, px);
                }
            }
            for i in 0..n {
                c.text_centered(margin + i * cw + cw / 2, margin - 14 * s, s, &format!("{}%", (i + 1) * 10), code8(170));
            }
        }
        Pattern::Sharpness => {
            c.fill(code8(128));
            let bw = w / 4;
            let bh = h / 2;
            let box_w = bw - 40 * s;
            let box_h = bh - 80 * s;
            let ox = 20 * s;
            let oy = 40 * s;
            for (i, title) in ["Lignes 1 px", "Lignes 1 px", "Damier 1 px", "Damier 2 px"].iter().enumerate() {
                let x = i as i32 * bw + ox;
                let y = oy;
                c.fill_rect(x, y, box_w, box_h, code8(0));
                match i {
                    0 => (0..box_w).step_by(2).for_each(|dx| c.fill_rect(x + dx, y, 1, box_h, code8(255))),
                    1 => (0..box_h).step_by(2).for_each(|dy| c.fill_rect(x, y + dy, box_w, 1, code8(255))),
                    _ => {
                        let cell = i as i32 - 1;
                        for dy in (0..box_h).step_by(cell as usize) {
                            for dx in (0..box_w).step_by(cell as usize) {
                                if ((dx / cell) + (dy / cell)) % 2 == 0 {
                                    c.fill_rect(x + dx, y + dy, cell, cell, code8(255));
                                }
                            }
                        }
                    }
                }
                c.text_centered(x + box_w / 2, y + box_h + 10 * s, s, title, code8(0));
            }
            let y = bh + 20 * s;
            c.fill_rect(ox, y, w - 2 * ox, h - y - 20 * s, code8(255));
            c.fill_rect(w / 2 - 100 * s, y + 20 * s, 200 * s, 40 * s, code8(0));
            let mut ty = y + 80 * s;
            for scale in 1..=3 {
                c.text(ox + 20 * s, ty, scale, "Netteté : aucun halo autour des lettres ni des bords. 0123456789", code8(0));
                ty += 12 * scale + 6 * s;
            }
        }
        Pattern::Uniformity => {
            c.fill(gray(UNIFORMITY_LEVELS[p] as f32 / 100.0));
        }
        Pattern::DeadPixels => {
            c.fill(Px::Code(DEAD_PIXEL_COLORS[p].1));
        }
        Pattern::HdrPeak10 => {
            c.fill(nits(0.0));
            let side = ((w as f32 * h as f32 * 0.1).sqrt()) as i32;
            let (x, y) = ((w - side) / 2, (h - side) / 2);
            c.fill_rect(x, y, side, side, nits(10000.0));
            let inner = side / 3;
            c.fill_rect(x + inner, y + inner, inner, inner, nits(PEAK_NITS[p]));
            c.text_centered(w / 2, y + side + 20 * s, s * 2, &format!("{} nits", PEAK_NITS[p]), gray(0.6));
        }
        Pattern::HdrPeakFull => {
            c.fill(nits(10000.0));
            let side = h / 3;
            c.fill_rect((w - side) / 2, (h - side) / 2, side, side, nits(PEAK_NITS[p]));
            c.text_centered(w / 2, (h + side) / 2 + 20 * s, s * 2, &format!("{} nits", PEAK_NITS[p]), gray(0.0));
        }
        Pattern::HdrBlack => {
            c.fill(nits(0.0));
            let side = h / 3;
            c.fill_rect((w - side) / 2, (h - side) / 2, side, side, nits(BLACK_NITS[p]));
            c.text_centered(w / 2, (h + side) / 2 + 30 * s, s * 2, &format!("{} nit", BLACK_NITS[p]), gray(0.35));
        }
        Pattern::HdrSteps => {
            c.fill(nits(0.0));
            let per_row = 13;
            let rows = STEP_NITS.len().div_ceil(per_row) as i32;
            let margin = 30 * s;
            let cw = (w - 2 * margin) / per_row as i32;
            let ch = (h - 2 * margin) / rows;
            for (i, v) in STEP_NITS.iter().enumerate() {
                let (r, col) = ((i / per_row) as i32, (i % per_row) as i32);
                let x = margin + col * cw;
                let y = margin + r * ch;
                c.fill_rect(x + 4, y + 4, cw - 8, ch - 50 * s, nits(*v));
                c.text_centered(x + cw / 2, y + ch - 40 * s, s, &format!("{v}"), gray(0.6));
            }
        }
        Pattern::HdrSdrWhite => {
            c.fill(nits(0.0));
            let third = w / 3;
            let top = h / 6;
            let bh = h * 2 / 3;
            c.fill_rect(0, top, third, bh, code8(255));
            c.fill_rect(third, top, third, bh, nits(203.0));
            c.fill_rect(third * 2, top, w - third * 2, bh, nits(100.0));
            let labels = [
                format!("Blanc SDR Windows ({:.0} nits)", c.sdr_white),
                "Référence HDR 203 nits".to_string(),
                "100 nits".to_string(),
            ];
            for (i, l) in labels.iter().enumerate() {
                c.text_centered(i as i32 * third + third / 2, top + bh + 20 * s, s, l, gray(0.6));
            }
        }
        Pattern::HdrGamut => {
            c.fill(nits(0.0));
            let white = GAMUT_WHITE[p];
            // Colonnes des matrices de conversion linéaire vers Rec.709 (blanc D65).
            let spaces: [(&str, [[f32; 3]; 3]); 3] = [
                ("Rec.709 / sRGB", [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]),
                ("DCI-P3", [[1.2249, -0.0420, -0.0197], [-0.2247, 1.0419, -0.0786], [0.0, 0.0, 1.0979]]),
                ("Rec.2020", [[1.6605, -0.1246, -0.0182], [-0.5876, 1.1329, -0.1006], [-0.0728, -0.0083, 1.1187]]),
            ];
            let margin = 40 * s;
            let cw = (w - 2 * margin) / 3;
            let ch = (h - 2 * margin - 30 * s) / 3;
            for (col, (name, m)) in spaces.iter().enumerate() {
                let x = margin + col as i32 * cw;
                c.text_centered(x + cw / 2, margin, s * 2, name, gray(0.6));
                for (row, primary) in m.iter().enumerate() {
                    let y = margin + 30 * s + row as i32 * ch;
                    c.fill_rect(x + 6 * s, y + 6 * s, cw - 12 * s, ch - 12 * s, Px::Nits(primary.map(|v| v * white)));
                }
            }
        }
        Pattern::HdrGradient => {
            c.fill(nits(0.0));
            let max_e = pq_encode(GRADIENT_MAX[p]);
            let bands = [[1.0, 1.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0f32]];
            let bh = h / bands.len() as i32;
            for (b, col) in bands.iter().enumerate() {
                for x in 0..w {
                    let n = pq_decode(x as f32 / (w - 1) as f32 * max_e);
                    c.fill_rect(x, b as i32 * bh, 1, bh, Px::Nits(col.map(|m| m * n)));
                }
            }
        }
    }
}

fn draw_level_rows(c: &mut Canvas, rows: &[Vec<u8>; 2], label: Px, s: i32) {
    let (w, h) = (c.w, c.h);
    let n = rows.iter().map(|r| r.len()).max().unwrap_or(1) as i32;
    let size = (w / (n + 2)).min(h / 4);
    let gap = (w - size * n) / (n + 1);
    for (r, row) in rows.iter().enumerate() {
        let y = h / 2 - size - 40 * s + r as i32 * (size + 80 * s);
        for (i, v) in row.iter().enumerate() {
            let x = gap + i as i32 * (size + gap);
            c.fill_rect(x, y, size, size, code8(*v));
            c.text_centered(x + size / 2, y + size + 12 * s, s, &v.to_string(), label);
        }
    }
}
