# Screen Calibration

Application Windows de calibration d'écran, écrite en Rust. Elle analyse votre écran (SDR et HDR), affiche des mires de test plein écran au pixel près et vous guide pas à pas pour régler luminosité, contraste, gamma, balance des blancs et luminances HDR.

> **Licence propriétaire — tous droits réservés.** Le code est consultable, mais la vente, la modification, la redistribution et la réutilisation sont interdites. Voir [LICENSE](LICENSE).

## Téléchargement

Récupérez `ScreenCalibration.exe` dans la page [Releases](../../releases/latest), puis lancez-le. Aucune installation et aucune fenêtre console.

Configuration requise : Windows 10 ou 11 (64 bits), carte graphique compatible DirectX 11.

## Fonctionnalités

### Informations écran
- Résolution, fréquence, profondeur de couleur (bits par canal), carte graphique.
- État HDR : pris en charge, activé, couleurs étendues (WCG), luminosité du contenu SDR en HDR.
- Données annoncées par l'écran (EDID) : luminances min / max / plein écran, primaires, point blanc.
- Couverture estimée des gamuts sRGB, DCI-P3 et Rec.2020.
- Profil ICC actif et diagnostic automatique (HDR désactivé, sortie 8 bits en HDR, large gamut sans profil, etc.).

### Mires de test (plein écran, rendu au pixel physique)
Affichage via Direct3D 11 : codes 8 bits exacts en SDR, scRGB 16 bits flottants en HDR (luminances absolues en nits).

| SDR | HDR |
| --- | --- |
| Niveau de noir (luminosité) | Luminance de crête (fenêtre 10 %) |
| Écrêtage du blanc (contraste) | Luminance plein écran (ABL) |
| Gamma par canal (1.8 à 2.6) | Luminance minimale (noir) |
| Échelle de gris (balance des blancs) | Paliers 0,05 → 10 000 nits (tone mapping) |
| Dégradés (banding) | Blanc SDR vs référence HDR 203 nits |
| Barres de couleurs | Gamut Rec.709 / DCI-P3 / Rec.2020 |
| Saturation | Dégradés PQ |
| Netteté | |
| Uniformité / fuites de lumière | |
| Pixels morts / bloqués | |

Chaque mire est accompagnée d'instructions détaillées. Commandes dans la mire : **flèches / clic** pour changer de mire, **haut/bas / molette** pour le réglage, **H** pour masquer le texte, **Échap** pour quitter.

### Correction gamma (SDR)
- Correction du gamma, du gain (blancs) et du noir par canal R/V/B via la LUT de la carte graphique.
- Aperçu des courbes, application en direct, sauvegarde par écran.
- Réapplication automatique à l'ouverture de session Windows (optionnelle).

### HDR
- Saisie des valeurs mesurées (crête, plein écran, noir) avec reprise directe depuis la mire.
- Recommandations pour les jeux, HGIG, curseur SDR de Windows.
- Raccourcis vers Windows HDR Calibration et les paramètres HDR.

## Méthode conseillée

1. Laissez l'écran chauffer 20 à 30 minutes, dans l'éclairage habituel de la pièce.
2. Réinitialisez l'écran, choisissez un mode d'image neutre (Standard / sRGB / Utilisateur), désactivez contraste dynamique, mode éco et Éclairage nocturne.
3. Suivez les mires SDR dans l'ordre en réglant d'abord l'écran lui-même (menu OSD).
4. Corrigez les écarts restants dans l'onglet « Correction gamma ».
5. En HDR : mesurez crête, plein écran et noir, puis reportez les valeurs dans Windows HDR Calibration et dans vos jeux.

## Limites

- En HDR, Windows ignore la LUT gamma : l'application mesure et guide, la correction se fait dans l'écran, Windows HDR Calibration et les jeux.
- Une calibration à l'œil corrige bien les défauts visibles, mais pas les valeurs absolues. Pour un travail colorimétrique exigeant, utilisez une sonde avec DisplayCAL / ArgyllCMS.
- Si Windows refuse une correction trop forte, la solution est décrite dans l'onglet « Aide & conseils ».

## Données enregistrées

Les réglages sont stockés dans `%APPDATA%\ScreenCalibration\config.json`. Le lancement automatique utilise la clé `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` (valeur `ScreenCalibration`), uniquement si vous l'activez.

## Compilation

Réservée à l'usage personnel autorisé par la [licence](LICENSE).

```powershell
cargo build --release
```

## Licence

Copyright (c) 2026 MasterBougli. Tous droits réservés. Voir [LICENSE](LICENSE).
