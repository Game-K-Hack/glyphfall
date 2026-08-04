//! Les bruitages, **synthétisés au démarrage** plutôt qu'embarqués.
//!
//! Un blip 8-bit est une onde carrée avec une enveloppe qui retombe : le
//! décrire en quelques lignes coûte moins qu'un fichier audio, et rien ne peut
//! diverger entre le son et le style graphique. Cela évite aussi d'ajouter
//! plusieurs centaines de kilooctets au binaire.
//!
//! Si le périphérique audio est indisponible — machine sans carte son, onglet
//! muet — les sons valent `None` et le jeu reste parfaitement jouable.

use macroquad::audio::{Sound, load_sound_from_bytes, play_sound_once};

/// Fréquence d'échantillonnage. 22 050 Hz suffit largement pour des blips et
/// divise par deux le temps de synthèse au démarrage.
const SAMPLE_RATE: u32 = 22_050;

pub struct Sfx {
    /// Un glyphe reconnu.
    hit: Option<Sound>,
    /// Une réponse qui ne correspond à aucune tuile.
    wrong: Option<Sound>,
    /// Une tuile franchit la ligne : une vie en moins.
    missed: Option<Sound>,
    /// Déplacement dans un menu.
    navigate: Option<Sound>,
    /// Validation d'un choix, lancement d'une manche.
    confirm: Option<Sound>,
}

impl Sfx {
    pub async fn load() -> Self {
        Self {
            // Deux notes qui montent : la récompense se lit à l'oreille.
            hit: tone(&[(660.0, 0.04), (990.0, 0.06)], 0.35).await,
            // Grave et court, sans être punitif.
            wrong: tone(&[(150.0, 0.10)], 0.30).await,
            // Deux notes qui descendent, le contraire exact de `hit`.
            missed: tone(&[(440.0, 0.06), (220.0, 0.12)], 0.35).await,
            navigate: tone(&[(520.0, 0.03)], 0.20).await,
            confirm: tone(&[(523.0, 0.05), (784.0, 0.05), (1046.0, 0.09)], 0.30).await,
        }
    }

    pub fn hit(&self) {
        play(&self.hit);
    }

    pub fn wrong(&self) {
        play(&self.wrong);
    }

    pub fn missed(&self) {
        play(&self.missed);
    }

    pub fn navigate(&self) {
        play(&self.navigate);
    }

    pub fn confirm(&self) {
        play(&self.confirm);
    }
}

fn play(sound: &Option<Sound>) {
    if let Some(sound) = sound {
        play_sound_once(sound);
    }
}

/// Synthétise une suite de notes en onde carrée et la charge comme un son.
///
/// `notes` associe une fréquence en hertz à une durée en secondes.
async fn tone(notes: &[(f32, f32)], volume: f32) -> Option<Sound> {
    let mut samples = Vec::new();

    for (frequency, duration) in notes {
        let count = (SAMPLE_RATE as f32 * duration) as usize;
        let period = SAMPLE_RATE as f32 / frequency;

        for index in 0..count {
            // Onde carrée : le signal ne prend que deux valeurs, ce qui donne
            // le timbre dur des puces sonores 8-bit.
            let square = if (index as f32 % period) < period / 2.0 { 1.0 } else { -1.0 };

            // Enveloppe descendante, pour que la note s'éteigne au lieu de se
            // couper net — une coupure franche produit un « clic ».
            let fade = 1.0 - index as f32 / count as f32;

            samples.push(square * fade * volume);
        }
    }

    load_sound_from_bytes(&encode_wav(&samples)).await.ok()
}

/// Emballe des échantillons dans un WAV PCM 16 bits mono, le format que
/// macroquad sait lire sur toutes les plateformes.
fn encode_wav(samples: &[f32]) -> Vec<u8> {
    const HEADER_SIZE: u32 = 36;
    const BITS_PER_SAMPLE: u16 = 16;
    const CHANNELS: u16 = 1;

    let data_size = (samples.len() * 2) as u32;
    let byte_rate = SAMPLE_RATE * CHANNELS as u32 * (BITS_PER_SAMPLE / 8) as u32;

    let mut wav = Vec::with_capacity(HEADER_SIZE as usize + 8 + data_size as usize);

    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(HEADER_SIZE + data_size).to_le_bytes());
    wav.extend_from_slice(b"WAVE");

    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes()); // taille du bloc fmt
    wav.extend_from_slice(&1u16.to_le_bytes()); // 1 = PCM non compressé
    wav.extend_from_slice(&CHANNELS.to_le_bytes());
    wav.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&(CHANNELS * BITS_PER_SAMPLE / 8).to_le_bytes()); // alignement
    wav.extend_from_slice(&BITS_PER_SAMPLE.to_le_bytes());

    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_size.to_le_bytes());
    for sample in samples {
        let value = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        wav.extend_from_slice(&value.to_le_bytes());
    }

    wav
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wav_header_describes_the_data_that_follows() {
        let samples = vec![0.0; 100];

        let wav = encode_wav(&samples);

        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[36..40], b"data");
        assert_eq!(wav.len(), 44 + samples.len() * 2, "en-tête de 44 octets puis 16 bits par échantillon");

        let announced = u32::from_le_bytes(wav[40..44].try_into().unwrap());
        assert_eq!(announced as usize, samples.len() * 2);

        let riff_size = u32::from_le_bytes(wav[4..8].try_into().unwrap());
        assert_eq!(riff_size as usize, wav.len() - 8, "RIFF annonce tout ce qui le suit");
    }

    #[test]
    fn samples_are_clamped_rather_than_wrapped() {
        // Sans bornage, un échantillon au-delà de 1.0 repasserait en négatif à
        // la conversion et produirait un craquement.
        let wav = encode_wav(&[2.0, -2.0]);

        let first = i16::from_le_bytes(wav[44..46].try_into().unwrap());
        let second = i16::from_le_bytes(wav[46..48].try_into().unwrap());

        assert_eq!(first, i16::MAX);
        assert_eq!(second, -i16::MAX);
    }
}
