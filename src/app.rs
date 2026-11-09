use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui::{
    self, CentralPanel, Color32, ComboBox, Grid, Panel, RichText, ScrollArea, Sense, Shape, Slider, Stroke, Vec2,
    ViewportCommand, WindowLevel,
};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::{PCWSTR, w};

use crate::config::{self, Config};
use crate::display::{self, MonitorInfo};
use crate::gamma::{self, GammaSettings};
use crate::pattern_window::{self, Shared, SharedState};
use crate::patterns::{BLACK_NITS, PEAK_NITS, Pattern};

const OK: Color32 = Color32::from_rgb(110, 200, 120);
const WARN: Color32 = Color32::from_rgb(230, 180, 80);
const ERR: Color32 = Color32::from_rgb(230, 100, 100);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Screen,
    Patterns,
    Gamma,
    Hdr,
    Help,
}

pub struct App {
    monitors: Vec<MonitorInfo>,
    selected: usize,
    tab: Tab,
    config: Config,
    shared: SharedState,
    status: Option<(bool, String)>,
    live: bool,
    on_top: bool,
    last_refresh: Instant,
}

impl App {
    pub fn new() -> Self {
        Self {
            monitors: display::enumerate(),
            selected: 0,
            tab: Tab::Screen,
            config: Config::load(),
            shared: Arc::new(Mutex::new(Shared::default())),
            status: None,
            live: false,
            on_top: false,
            last_refresh: Instant::now(),
        }
    }

    fn refresh(&mut self) {
        let current = self.monitors.get(self.selected).map(|m| m.gdi_name.clone());
        self.monitors = display::enumerate();
        self.selected = current
            .and_then(|name| self.monitors.iter().position(|m| m.gdi_name == name))
            .unwrap_or(0);
        self.last_refresh = Instant::now();
    }

    fn set_status(&mut self, result: Result<(), String>, ok_message: &str) {
        self.status = Some(match result {
            Ok(()) => (true, ok_message.to_string()),
            Err(e) => (false, e),
        });
    }

    fn save(&mut self) {
        let result = self.config.save();
        self.set_status(result, "Réglages enregistrés.");
    }
}

fn open_uri(uri: &str) {
    let wide: Vec<u16> = uri.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        ShellExecuteW(None, w!("open"), PCWSTR(wide.as_ptr()), PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL);
    }
}

fn yes_no(v: bool) -> RichText {
    if v { RichText::new("Oui").color(OK) } else { RichText::new("Non").color(Color32::GRAY) }
}

fn nits_text(v: f32) -> String {
    if v <= 0.0 {
        return "non renseigné".into();
    }
    let n = format!("{v:.4}");
    format!("{} nits", n.trim_end_matches('0').trim_end_matches('.'))
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let (open, error) = {
            let st = self.shared.lock().unwrap();
            (st.open, st.error.clone())
        };
        if let Some(e) = error {
            self.status = Some((false, e));
            self.shared.lock().unwrap().error = None;
        }
        if open != self.on_top {
            let level = if open { WindowLevel::AlwaysOnTop } else { WindowLevel::Normal };
            ctx.send_viewport_cmd(ViewportCommand::WindowLevel(level));
            self.on_top = open;
        }
        if open {
            ctx.request_repaint_after(Duration::from_millis(100));
        } else {
            if self.last_refresh.elapsed() > Duration::from_secs(3) {
                self.refresh();
            }
            ctx.request_repaint_after(Duration::from_secs(3));
        }

        Panel::left("nav").resizable(false).exact_size(230.0).show(ui, |ui| {
            ui.add_space(8.0);
            ui.heading("Calibration écran");
            ui.add_space(8.0);
            ui.label("Écran :");
            let label = self.monitors.get(self.selected).map(|m| m.label()).unwrap_or_else(|| "Aucun écran".into());
            ComboBox::from_id_salt("monitor").width(210.0).selected_text(label).show_ui(ui, |ui| {
                for (i, m) in self.monitors.iter().enumerate() {
                    ui.selectable_value(&mut self.selected, i, m.label());
                }
            });
            if ui.button("⟳ Actualiser").clicked() {
                self.refresh();
            }
            ui.separator();
            for (tab, name) in [
                (Tab::Screen, "🖥 Informations écran"),
                (Tab::Patterns, "🎯 Mires & guide"),
                (Tab::Gamma, "📈 Correction gamma (SDR)"),
                (Tab::Hdr, "☀ HDR"),
                (Tab::Help, "❓ Aide & conseils"),
            ] {
                ui.selectable_value(&mut self.tab, tab, RichText::new(name).size(15.0));
            }
            ui.separator();
            if open {
                ui.colored_label(OK, "Mires affichées");
                if ui.button("Fermer les mires").clicked() {
                    self.shared.lock().unwrap().close_request = true;
                }
            } else if let Some(m) = self.monitors.get(self.selected).cloned() {
                if ui.button(RichText::new("▶ Afficher les mires").size(15.0)).clicked() {
                    pattern_window::open(self.shared.clone(), m);
                }
            }
        });

        Panel::bottom("status").show(ui, |ui| {
            ui.add_space(4.0);
            match &self.status {
                Some((true, msg)) => ui.colored_label(OK, msg),
                Some((false, msg)) => ui.colored_label(ERR, msg),
                None => ui.label("Prêt."),
            };
            ui.add_space(4.0);
        });

        CentralPanel::default().show(ui, |ui| {
            ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                let Some(mon) = self.monitors.get(self.selected).cloned() else {
                    ui.label("Aucun écran détecté.");
                    return;
                };
                match self.tab {
                    Tab::Screen => self.screen_tab(ui, &mon),
                    Tab::Patterns => self.patterns_tab(ui, &mon, open),
                    Tab::Gamma => self.gamma_tab(ui, &mon),
                    Tab::Hdr => self.hdr_tab(ui, &mon),
                    Tab::Help => help_tab(ui),
                }
            });
        });
    }
}

impl App {
    fn screen_tab(&mut self, ui: &mut egui::Ui, m: &MonitorInfo) {
        ui.heading(if m.friendly_name.is_empty() { &m.gdi_name } else { &m.friendly_name });
        ui.add_space(6.0);
        Grid::new("info").num_columns(2).striped(true).spacing([24.0, 6.0]).show(ui, |ui| {
            let mut row = |k: &str, v: RichText| {
                ui.label(k);
                ui.label(v);
                ui.end_row();
            };
            row("Identifiant Windows", m.gdi_name.clone().into());
            row("Carte graphique", m.adapter_name.clone().into());
            row("Résolution", format!("{} × {}", m.width(), m.height()).into());
            row("Fréquence", format!("{} Hz", m.refresh_hz).into());
            row("Profondeur de couleur", format!("{} bits par canal", m.bits_per_color).into());
            row("Espace de sortie", m.color_space.clone().into());
            row("HDR pris en charge", yes_no(m.advanced_color_supported));
            row("HDR activé", yes_no(m.hdr_active));
            row("Couleurs étendues (WCG) forcées", yes_no(m.wide_color_enforced));
            if let Some(n) = m.sdr_white_nits {
                row("Luminosité du contenu SDR", format!("{n:.0} nits").into());
            }
            row(
                "Profil ICC Windows",
                m.icc_profile.clone().unwrap_or_else(|| "aucun / sRGB par défaut".into()).into(),
            );
            row("Luminance max annoncée (EDID)", nits_text(m.max_luminance).into());
            row("Luminance plein écran annoncée", nits_text(m.max_full_frame_luminance).into());
            row("Luminance min annoncée", nits_text(m.min_luminance).into());
            row(
                "Primaires (x, y) R / G / B",
                format!(
                    "({:.3}, {:.3})  ({:.3}, {:.3})  ({:.3}, {:.3})",
                    m.red_primary[0],
                    m.red_primary[1],
                    m.green_primary[0],
                    m.green_primary[1],
                    m.blue_primary[0],
                    m.blue_primary[1]
                )
                .into(),
            );
            row("Point blanc (x, y)", format!("({:.4}, {:.4})", m.white_point[0], m.white_point[1]).into());
            if let Some((srgb, p3, bt2020)) = m.gamut_coverage() {
                row(
                    "Couverture gamut (selon EDID)",
                    format!("sRGB {:.0} %   DCI-P3 {:.0} %   Rec.2020 {:.0} %", srgb * 100.0, p3 * 100.0, bt2020 * 100.0)
                        .into(),
                );
            }
        });

        ui.add_space(12.0);
        ui.heading("Diagnostic");
        let mut notes: Vec<(Color32, String)> = Vec::new();
        if m.bits_per_color > 0 && m.bits_per_color < 8 {
            notes.push((ERR, "Sortie en moins de 8 bits : du banding est probable. Vérifiez câble/fréquence.".into()));
        } else if m.bits_per_color == 8 && m.hdr_active {
            notes.push((WARN, "HDR en 8 bits : Windows applique un dithering, mais 10 bits est préférable \
                (réduire la fréquence ou utiliser DisplayPort / HDMI 2.1 si nécessaire).".into()));
        }
        if m.advanced_color_supported && !m.hdr_active {
            notes.push((WARN, "L'écran prend en charge le HDR mais il est désactivé (Win+Alt+B pour basculer). \
                Calibrez d'abord en SDR, puis en HDR : ce sont deux réglages distincts de l'écran.".into()));
        }
        if m.advanced_color_force_disabled {
            notes.push((WARN, "La couleur avancée est désactivée de force (pilote ou stratégie).".into()));
        }
        if let Some((srgb, p3, _)) = m.gamut_coverage() {
            if p3 > 0.85 && !m.hdr_active && m.icc_profile.is_none() {
                notes.push((WARN, format!(
                    "Écran à large gamut (~{:.0} % DCI-P3) sans profil ICC : en SDR, les couleurs des applications \
                     non gérées seront sursaturées. Utilisez le mode sRGB de l'écran ou activez la « Gestion automatique \
                     des couleurs » (Windows 11) si disponible.", p3 * 100.0)));
            } else if srgb < 0.9 {
                notes.push((WARN, "Gamut inférieur au sRGB d'après l'EDID : les couleurs saturées seront limitées.".into()));
            }
        }
        if m.max_luminance <= 0.0 {
            notes.push((Color32::GRAY, "L'écran ne communique pas ses luminances (EDID incomplet) : utilisez les mires HDR pour les mesurer.".into()));
        }
        if notes.is_empty() {
            notes.push((OK, "Aucun problème détecté.".into()));
        }
        for (color, text) in notes {
            ui.colored_label(color, format!("• {text}"));
        }
        ui.add_space(6.0);
        ui.small("Les valeurs annoncées proviennent de l'EDID de l'écran et sont souvent approximatives ; \
                  les mires permettent de vérifier les valeurs réelles.");
    }

    fn patterns_tab(&mut self, ui: &mut egui::Ui, m: &MonitorInfo, open: bool) {
        ui.heading("Mires de test et guide pas à pas");
        egui::CollapsingHeader::new("Avant de commencer (important)").default_open(!open).show(ui, |ui| {
            for line in [
                "Laissez l'écran chauffer 20–30 minutes.",
                "Éclairage de la pièce identique à votre usage habituel, sans reflet direct sur l'écran.",
                "Résolution native et mode d'image « Standard », « sRGB » ou « Utilisateur » (pas « Dynamique »/« Vivid »).",
                "Désactivez : contraste dynamique, mode éco, capteur de luminosité ambiante, Éclairage nocturne de Windows.",
                "Plage RGB « Complète » dans le pilote graphique ET « Complète/Haute » dans l'écran (ou Limitée des deux côtés).",
                "Réinitialisez la correction gamma de cette application avant de mesurer.",
                "Suivez les étapes dans l'ordre : chaque réglage influence les suivants.",
            ] {
                ui.label(format!("• {line}"));
            }
        });
        ui.add_space(6.0);
        if m.hdr_active {
            ui.colored_label(WARN, format!(
                "HDR actif : les mires SDR sont affichées comme Windows affiche le contenu SDR (blanc = {:.0} nits). \
                 Pour régler l'écran en SDR, désactivez le HDR (Win+Alt+B) ; les mires HDR ci-dessous sont disponibles.",
                m.sdr_white_nits.unwrap_or(80.0)
            ));
        }

        let (current, param) = {
            let st = self.shared.lock().unwrap();
            (st.pattern, st.param())
        };
        ui.columns(2, |cols| {
            let ui = &mut cols[0];
            ui.strong("Étapes SDR");
            for (i, p) in Pattern::SDR.iter().enumerate() {
                if ui.selectable_label(current == *p, format!("{}. {}", i + 1, p.title())).clicked() {
                    self.shared.lock().unwrap().set_pattern(*p);
                }
            }
            ui.add_space(8.0);
            ui.strong("Étapes HDR");
            ui.add_enabled_ui(m.hdr_active, |ui| {
                for (i, p) in Pattern::HDR.iter().enumerate() {
                    if ui.selectable_label(current == *p, format!("{}. {}", i + 1, p.title())).clicked() {
                        self.shared.lock().unwrap().set_pattern(*p);
                    }
                }
            });
            if !m.hdr_active {
                ui.small("Activez le HDR de Windows sur cet écran pour accéder aux mires HDR.");
            }

            let ui = &mut cols[1];
            ui.heading(current.title());
            ui.add_space(4.0);
            ui.label(current.instructions());
            if let Some(label) = current.param_label(param) {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("◀").clicked() {
                        self.shared.lock().unwrap().step_param(-1);
                    }
                    ui.label(label);
                    if ui.button("▶").clicked() {
                        self.shared.lock().unwrap().step_param(1);
                    }
                });
            }
            ui.add_space(10.0);
            if open {
                ui.horizontal(|ui| {
                    if ui.button("⏮ Mire précédente").clicked() {
                        self.shared.lock().unwrap().step_pattern(-1);
                    }
                    if ui.button("Mire suivante ⏭").clicked() {
                        self.shared.lock().unwrap().step_pattern(1);
                    }
                });
                ui.small("Cette fenêtre reste au premier plan : déplacez-la sur le côté. Dans la mire : flèches, \
                          clic, molette, H pour masquer le texte, Échap pour quitter.");
            } else if (!current.is_hdr() || m.hdr_active)
                && ui.button(RichText::new("▶ Afficher cette mire en plein écran").size(15.0)).clicked()
            {
                pattern_window::open(self.shared.clone(), m.clone());
            }
        });
    }

    fn gamma_tab(&mut self, ui: &mut egui::Ui, m: &MonitorInfo) {
        ui.heading("Correction gamma et balance des blancs (SDR)");
        ui.label("Modifie la table de correction (LUT) de la carte graphique pour cet écran. À utiliser après avoir \
                  réglé au mieux l'écran lui-même (luminosité, contraste, température, gamma) : les réglages de \
                  l'écran sont toujours préférables car ils ne réduisent pas le nombre de nuances.");
        if m.hdr_active {
            ui.colored_label(WARN, "Le HDR est actif : Windows ignore la LUT gamma en HDR. Désactivez le HDR pour \
                                    utiliser cette correction, ou utilisez l'onglet HDR.");
        }
        ui.add_space(6.0);
        ui.label(RichText::new("Méthode : 1) Réinitialiser  2) Mire « Gamma » canal par canal → saisir les gammas mesurés  \
                                3) Mire « Échelle de gris » → corriger les dominantes avec les gains (clairs) et le noir (sombres).").italics());
        ui.add_space(8.0);

        let gdi = m.gdi_name.clone();
        let before = self.config.monitor(&gdi).gamma.clone();
        let mut settings = before.clone();

        Grid::new("gamma").num_columns(4).spacing([16.0, 8.0]).show(ui, |ui| {
            ui.label("");
            ui.strong("Gamma mesuré");
            ui.strong("Gain (blanc)");
            ui.strong("Noir");
            ui.end_row();
            for (c, (name, color)) in [
                ("Rouge", Color32::from_rgb(240, 90, 90)),
                ("Vert", Color32::from_rgb(90, 220, 90)),
                ("Bleu", Color32::from_rgb(100, 140, 255)),
            ]
            .into_iter()
            .enumerate()
            {
                ui.colored_label(color, name);
                ui.add(Slider::new(&mut settings.measured_gamma[c], 1.6..=3.0).step_by(0.01));
                ui.add(Slider::new(&mut settings.gain[c], 0.5..=1.0).step_by(0.005));
                ui.add(Slider::new(&mut settings.black[c], 0.0..=0.1).step_by(0.001));
                ui.end_row();
            }
        });
        ui.horizontal(|ui| {
            ui.label("Tous les canaux :");
            let mut all = settings.measured_gamma[1];
            if ui.add(Slider::new(&mut all, 1.6..=3.0).step_by(0.01).text("gamma mesuré (gris)")).changed() {
                settings.measured_gamma = [all; 3];
            }
        });
        ui.horizontal(|ui| {
            ui.label("Gamma cible :");
            ui.add(Slider::new(&mut settings.target_gamma, 1.8..=2.6).step_by(0.01));
            for g in [2.2f32, 2.4] {
                if ui.button(format!("{g}")).clicked() {
                    settings.target_gamma = g;
                }
            }
        });

        ui.add_space(8.0);
        let (response, painter) = ui.allocate_painter(Vec2::new(260.0, 260.0), Sense::hover());
        let rect = response.rect;
        painter.rect_filled(rect, 4.0, Color32::from_gray(20));
        painter.line_segment([rect.left_bottom(), rect.right_top()], Stroke::new(1.0, Color32::from_gray(70)));
        for (c, color) in [Color32::from_rgb(240, 90, 90), Color32::from_rgb(90, 220, 90), Color32::from_rgb(100, 140, 255)]
            .into_iter()
            .enumerate()
        {
            let points = (0..=64)
                .map(|i| {
                    let x = i as f32 / 64.0;
                    let y = settings.curve(c, x);
                    egui::pos2(rect.left() + x * rect.width(), rect.bottom() - y * rect.height())
                })
                .collect();
            painter.add(Shape::line(points, Stroke::new(1.5, color)));
        }
        ui.small("Courbes de correction appliquées (entrée → sortie).");

        ui.add_space(8.0);
        ui.checkbox(&mut self.live, "Appliquer en direct pendant les réglages");
        let mut apply_now = self.live && settings != before;
        ui.horizontal(|ui| {
            if ui.button("Appliquer").clicked() {
                apply_now = true;
            }
            if ui.button("Réinitialiser (LUT neutre)").clicked() {
                settings = GammaSettings::default();
                let result = gamma::reset(&gdi);
                self.set_status(result, "Correction réinitialisée.");
            }
            if ui.button("💾 Enregistrer").clicked() {
                self.config.monitor(&gdi).gamma = settings.clone();
                self.save();
            }
        });
        if apply_now {
            let result = gamma::apply(&gdi, &settings);
            self.set_status(result, "Correction appliquée.");
        }
        self.config.monitor(&gdi).gamma = settings;

        ui.add_space(8.0);
        let mut startup = self.config.monitor(&gdi).apply_at_startup;
        if ui.checkbox(&mut startup, "Réappliquer cette correction à l'ouverture de session Windows").changed() {
            self.config.monitor(&gdi).apply_at_startup = startup;
            let any = self.config.monitors.values().any(|c| c.apply_at_startup);
            let result = config::set_autostart(any).and_then(|_| self.config.save());
            self.set_status(result, if startup { "Correction appliquée au démarrage." } else { "Démarrage automatique mis à jour." });
        }
        if self.config.monitor(&gdi).apply_at_startup && !config::autostart_enabled() {
            ui.colored_label(WARN, "Entrée de démarrage absente : recochez la case.");
        }
    }

    fn hdr_tab(&mut self, ui: &mut egui::Ui, m: &MonitorInfo) {
        ui.heading("HDR");
        if !m.advanced_color_supported {
            ui.label("Cet écran ne signale pas de prise en charge HDR à Windows. Si c'est un écran HDR, vérifiez le câble \
                      (HDMI 2.0+/DisplayPort 1.4), le mode HDR / la plage d'entrée de l'écran et le pilote graphique.");
            return;
        }
        if !m.hdr_active {
            ui.colored_label(WARN, "Le HDR est pris en charge mais désactivé sur cet écran.");
            ui.label("Activez-le (Win+Alt+B ou Paramètres > Affichage > HDR) pour mesurer ses capacités réelles.");
            if ui.button("Ouvrir les paramètres HDR de Windows").clicked() {
                open_uri("ms-settings:display-hdr");
            }
            return;
        }

        ui.label("Mesurez les capacités réelles de l'écran avec les mires HDR (onglet « Mires & guide »), puis notez \
                  les résultats ici. Ces valeurs servent aux réglages HDR des jeux et à l'application Windows HDR Calibration.");
        ui.add_space(8.0);

        let gdi = m.gdi_name.clone();
        let (current, param) = {
            let st = self.shared.lock().unwrap();
            (st.pattern, st.param())
        };
        let mut changed = false;
        let mut hdr = self.config.monitor(&gdi).hdr.clone();
        Grid::new("hdr").num_columns(4).striped(true).spacing([18.0, 8.0]).show(ui, |ui| {
            ui.strong("Mesure");
            ui.strong("Annoncé (EDID)");
            ui.strong("Mesuré");
            ui.label("");
            ui.end_row();
            let rows: [(&str, f32, &mut Option<f32>, Pattern, f32); 3] = [
                ("Luminance de crête (10 %)", m.max_luminance, &mut hdr.peak_10, Pattern::HdrPeak10, PEAK_NITS[param.min(PEAK_NITS.len() - 1)]),
                ("Luminance plein écran", m.max_full_frame_luminance, &mut hdr.peak_full, Pattern::HdrPeakFull, PEAK_NITS[param.min(PEAK_NITS.len() - 1)]),
                ("Noir minimal", m.min_luminance, &mut hdr.min_black, Pattern::HdrBlack, BLACK_NITS[param.min(BLACK_NITS.len() - 1)]),
            ];
            for (name, edid, value, pattern, from_pattern) in rows {
                ui.label(name);
                ui.label(nits_text(edid));
                let mut v = value.unwrap_or(0.0);
                let speed = if pattern == Pattern::HdrBlack { 0.001 } else { 10.0 };
                if ui.add(egui::DragValue::new(&mut v).speed(speed).range(0.0..=10000.0).suffix(" nits")).changed() {
                    *value = Some(v);
                    changed = true;
                }
                if ui
                    .add_enabled(current == pattern, egui::Button::new("← valeur de la mire"))
                    .on_disabled_hover_text("Sélectionnez la mire correspondante")
                    .clicked()
                {
                    *value = Some(from_pattern);
                    changed = true;
                }
                ui.end_row();
            }
        });
        if changed {
            self.config.monitor(&gdi).hdr = hdr.clone();
            self.save();
        }

        ui.add_space(12.0);
        ui.heading("Recommandations");
        let peak = hdr.peak_10.unwrap_or(m.max_luminance);
        let full = hdr.peak_full.unwrap_or(m.max_full_frame_luminance);
        for line in [
            format!("Jeux : « luminance maximale » ≈ {peak:.0} nits, « luminance plein écran / papier blanc » ≤ {full:.0} nits, \
                     noir au plus bas sans écraser les détails."),
            "Écran/TV : activez HGIG (ou désactivez le « tone mapping dynamique ») pour que les valeurs mesurées soient \
             respectées ; désactivez le contraste dynamique et la luminosité automatique."
                .into(),
            format!("Contenu SDR en HDR : blanc SDR actuel = {:.0} nits. Réglez le curseur « Luminosité du contenu SDR » \
                     avec la mire « Blanc SDR vs référence HDR ».", m.sdr_white_nits.unwrap_or(80.0)),
            "Windows HDR Calibration (application Microsoft gratuite) enregistre ces valeurs dans un profil de couleur \
             utilisé par Windows, Auto HDR et les jeux : reportez-y les valeurs mesurées ici."
                .into(),
            "Activez la « Gestion automatique des couleurs » (Windows 11, si proposée) pour un rendu correct des couleurs SDR \
             sur un écran à large gamut."
                .into(),
        ] {
            ui.label(format!("• {line}"));
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("Ouvrir Windows HDR Calibration").clicked() {
                open_uri("ms-windows-store://pdp/?ProductId=9N7F2SM5D1LR");
            }
            if ui.button("Paramètres HDR de Windows").clicked() {
                open_uri("ms-settings:display-hdr");
            }
        });
    }
}

fn help_tab(ui: &mut egui::Ui) {
    ui.heading("Aide & conseils");
    let section = |ui: &mut egui::Ui, title: &str, lines: &[&str]| {
        ui.add_space(8.0);
        ui.strong(title);
        for l in lines {
            ui.label(format!("• {l}"));
        }
    };
    section(ui, "Ordre de calibration recommandé", &[
        "Réglages de l'écran (OSD) d'abord : mode d'image, luminosité, contraste, température 6500 K, gamma, netteté.",
        "Puis correction logicielle (onglet Correction gamma) uniquement pour les écarts restants.",
        "En HDR : mesurez crête, plein écran et noir, puis reportez-les dans Windows HDR Calibration et dans les jeux.",
    ]);
    section(ui, "Limites d'une calibration à l'œil", &[
        "L'œil s'adapte : il juge bien les écarts relatifs (gris neutre, nuances distinctes) mais pas les valeurs absolues.",
        "Pour une calibration précise (graphisme, photo, vidéo), utilisez une sonde colorimétrique (Calibrite, Datacolor) \
         avec DisplayCAL / ArgyllCMS qui créera un profil ICC.",
    ]);
    section(ui, "Si Windows refuse la correction gamma", &[
        "Windows limite par défaut l'amplitude des corrections. Pour lever la limite (administrateur requis) : créez la valeur \
         DWORD « GdiIcmGammaRange » = 256 dans HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\ICM, puis \
         redémarrez.",
        "Certains logiciels (pilote, utilitaires de profil ICC, f.lux) réécrivent la LUT : désactivez-les pour éviter les conflits.",
    ]);
    section(ui, "Autres réglages qui influencent les couleurs", &[
        "Pilote graphique : plage RGB, profondeur de couleur, format de sortie (RGB plutôt que YCbCr 4:2:2), vibrance/saturation \
         numérique à la valeur neutre.",
        "Windows : Éclairage nocturne, Gestion automatique des couleurs, profil ICC associé à l'écran (Paramètres > Affichage > \
         Profil de couleur).",
        "Écran : overdrive excessif (halos en mouvement), modes « jeu »/« FPS » qui relèvent les noirs, Black Equalizer.",
    ]);
}
