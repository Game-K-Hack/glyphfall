//! Les bruitages, **synthétisés au démarrage** plutôt qu'embarqués.
//!
//! Un blip 8-bit est une onde carrée avec une enveloppe qui retombe : le
//! décrire en quelques lignes coûte moins qu'un fichier audio, et rien ne peut
//! diverger entre le son et le style graphique. Cela évite aussi d'ajouter
//! plusieurs centaines de kilooctets au binaire.
//!
//! Si le périphérique audio est indisponible — machine sans carte son, onglet
//! muet — les sons valent `None` et le jeu reste parfaitement jouable.

use macroquad::audio::{PlaySoundParams, Sound, load_sound_from_bytes, play_sound};

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
    /// Multiplicateur venant des réglages, entre 0 et 1.
    volume: f32,
}

impl Sfx {
    pub async fn load(volume: f32) -> Self {
        Self {
            // Deux notes qui montent : la récompense se lit à l'oreille.
            hit: tone(&[(660.0, 0.04), (990.0, 0.06)], 0.35).await,
            // Grave et court, sans être punitif.
            wrong: tone(&[(150.0, 0.10)], 0.30).await,
            // Deux notes qui descendent, le contraire exact de `hit`.
            missed: tone(&[(440.0, 0.06), (220.0, 0.12)], 0.35).await,
            navigate: tone(&[(520.0, 0.03)], 0.20).await,
            confirm: tone(&[(523.0, 0.05), (784.0, 0.05), (1046.0, 0.09)], 0.30).await,
            volume,
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }

    pub fn hit(&self) {
        self.play(&self.hit);
    }

    pub fn wrong(&self) {
        self.play(&self.wrong);
    }

    pub fn missed(&self) {
        self.play(&self.missed);
    }

    pub fn navigate(&self) {
        self.play(&self.navigate);
    }

    pub fn confirm(&self) {
        self.play(&self.confirm);
    }

    fn play(&self, sound: &Option<Sound>) {
        // À volume nul, ne rien émettre du tout plutôt qu'un son inaudible :
        // le moteur audio n'a alors aucune voix à gérer.
        if self.volume <= 0.0 {
            return;
        }

        if let Some(sound) = sound {
            play_sound(sound, PlaySoundParams { looped: false, volume: self.volume });
        }
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

    load_sound_from_bytes(&encode_wav(&samples, SAMPLE_RATE, 1)).await.ok()
}

/// Taille de l'en-tête WAV canonique, en octets.
pub const WAV_HEADER_SIZE: usize = 44;

const BITS_PER_SAMPLE: u16 = 16;

/// Emballe des échantillons entrelacés dans un WAV PCM 16 bits, le format que
/// macroquad sait lire sur toutes les plateformes.
pub fn encode_wav(samples: &[f32], sample_rate: u32, channels: u16) -> Vec<u8> {
    let mut wav = vec![0; WAV_HEADER_SIZE];
    wav.reserve(samples.len() * 2);

    for sample in samples {
        wav.extend_from_slice(&to_pcm16(*sample).to_le_bytes());
    }

    write_wav_header(&mut wav, sample_rate, channels);
    wav
}

/// Convertit un échantillon flottant en entier 16 bits.
///
/// Le bornage évite qu'une valeur au-delà de 1.0 ne repasse en négatif à la
/// conversion, ce qui s'entendrait comme un craquement.
pub fn to_pcm16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
}

/// Écrit l'en-tête au début d'un tampon dont les 44 premiers octets ont été
/// réservés, le reste contenant déjà les données PCM.
///
/// Procéder ainsi permet d'encoder au fil du décodage d'un MP3, sans garder en
/// mémoire une copie flottante de tout le morceau en plus du résultat.
pub fn write_wav_header(wav: &mut [u8], sample_rate: u32, channels: u16) {
    debug_assert!(wav.len() >= WAV_HEADER_SIZE);

    let data_size = (wav.len() - WAV_HEADER_SIZE) as u32;
    let byte_rate = sample_rate * channels as u32 * (BITS_PER_SAMPLE / 8) as u32;

    let mut header = Vec::with_capacity(WAV_HEADER_SIZE);
    header.extend_from_slice(b"RIFF");
    // Tout ce qui suit ce champ, soit l'en-tête moins ses huit premiers octets.
    header.extend_from_slice(&((WAV_HEADER_SIZE - 8) as u32 + data_size).to_le_bytes());
    header.extend_from_slice(b"WAVE");

    header.extend_from_slice(b"fmt ");
    header.extend_from_slice(&16u32.to_le_bytes()); // taille du bloc fmt
    header.extend_from_slice(&1u16.to_le_bytes()); // 1 = PCM non compressé
    header.extend_from_slice(&channels.to_le_bytes());
    header.extend_from_slice(&sample_rate.to_le_bytes());
    header.extend_from_slice(&byte_rate.to_le_bytes());
    header.extend_from_slice(&(channels * BITS_PER_SAMPLE / 8).to_le_bytes()); // alignement
    header.extend_from_slice(&BITS_PER_SAMPLE.to_le_bytes());

    header.extend_from_slice(b"data");
    header.extend_from_slice(&data_size.to_le_bytes());

    wav[..WAV_HEADER_SIZE].copy_from_slice(&header);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wav_header_describes_the_data_that_follows() {
        let samples = vec![0.0; 100];

        let wav = encode_wav(&samples, SAMPLE_RATE, 1);

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
        let wav = encode_wav(&[2.0, -2.0], SAMPLE_RATE, 1);

        let first = i16::from_le_bytes(wav[44..46].try_into().unwrap());
        let second = i16::from_le_bytes(wav[46..48].try_into().unwrap());

        assert_eq!(first, i16::MAX);
        assert_eq!(second, -i16::MAX);
    }
}
