use macroquad::prelude::*;

/// Agrandissement de la fenêtre à l'ouverture.
const ZOOM: i32 = if crate::gfx::canvas::PORTRAIT { 2 } else { 3 };

pub fn window_conf() -> Conf {
    // 1. On charge tes fichiers (qu'ils soient en PNG, RAW ou autre)
    let icon16_bytes = include_bytes!("assets/icons/icon16.rgba");
    let icon32_bytes = include_bytes!("assets/icons/icon32.rgba");
    let icon64_bytes = include_bytes!("assets/icons/icon64.rgba");
    
    // 2. On prépare les conteneurs vides aux tailles strictes exigées par miniquad
    let mut small_array = [0u8; 16 * 16 * 4];   // 1024 octets
    let mut medium_array = [0u8; 32 * 32 * 4];  // 4096 octets
    let mut big_array = [0u8; 64 * 64 * 4];     // 16384 octets

    // 3. On copie intelligemment les données sans dépasser les limites
    let len_small = icon16_bytes.len().min(small_array.len());
    small_array[..len_small].copy_from_slice(&icon16_bytes[..len_small]);

    let len_medium = icon32_bytes.len().min(medium_array.len());
    medium_array[..len_medium].copy_from_slice(&icon32_bytes[..len_medium]);

    let len_big = icon64_bytes.len().min(big_array.len());
    big_array[..len_big].copy_from_slice(&icon64_bytes[..len_big]);

    // 4. On crée la structure finale
    let icon = miniquad::conf::Icon {
        small: small_array,
        medium: medium_array,
        big: big_array,
    };

    Conf {
        window_title: "Glyphfall".to_string(),
        // Un multiple entier de la toile : la fenêtre s'ouvre pile sur un
        // facteur d'agrandissement, sans bandes de letterbox. Le portrait se
        // contente de deux, trois fois trois cent quatre-vingt-quatre pixels
        // dépassant la hauteur d'un écran de bureau.
        window_width: crate::gfx::canvas::WIDTH as i32 * ZOOM,
        window_height: crate::gfx::canvas::HEIGHT as i32 * ZOOM,
        fullscreen: false,
        // Le high-DPI ferait rendre macroquad à une résolution non entière et
        // reviendrait à flouter ce que l'agrandissement au pixel garantit.
        high_dpi: false,
        icon: Some(icon),
        // WebGL 2 dans le navigateur, alors que miniquad demande WebGL 1 par
        // défaut.
        //
        // Le jeu dessine tout sur une toile virtuelle avant de l'agrandir à
        // l'écran, et cette cible de rendu passe par des appels — `readBuffer`,
        // `bindFramebuffer` sur les cibles de lecture — qui n'existent pas en
        // WebGL 1. La page se chargeait donc, restait noire, et lâchait
        // « gl.readBuffer is not a function » à la première image.
        //
        // Ce réglage est ignoré partout ailleurs.
        platform: miniquad::conf::Platform {
            webgl_version: miniquad::conf::WebGLVersion::WebGL2,
            ..Default::default()
        },
        ..Default::default()
    }
}