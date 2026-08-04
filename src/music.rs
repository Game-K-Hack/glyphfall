//! La musique de fond des menus : les fichiers déposés dans
//! `assets/music/menu/` sont enchaînés dans un ordre aléatoire.
//!
//! Deux contraintes façonnent ce module.
//!
//! La première : le moteur audio ne sait pas lire le MP3. Les morceaux sont
//! donc décodés ici puis réemballés en WAV brut avant de lui être confiés. Le
//! WAV et l'OGG passent par le même chemin bien que le moteur sache les lire :
//! c'est ce décodage qui donne la durée exacte du morceau, dont la playlist a
//! besoin pour savoir quand enchaîner.
//!
//! La seconde : un morceau décodé occupe beaucoup de mémoire — une minute de
//! stéréo à 44,1 kHz fait une vingtaine de mégaoctets. Les pistes ne sont donc
//! décodées qu'au moment de les jouer, une seule à la fois, et l'ancienne est
//! libérée avant que la suivante ne soit chargée.

use std::io::Cursor;

use include_dir::{Dir, File, include_dir};
use macroquad::audio::{
    PlaySoundParams, Sound, load_sound_from_bytes, set_sound_volume, stop_sound,
};
use macroquad::prelude::rand;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::audio::{WAV_HEADER_SIZE, to_pcm16, write_wav_header};

static MENU_MUSIC: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/assets/music/menu");
static GAME_MUSIC: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/assets/music/game");

/// Les extensions reconnues. Tout le reste du dossier est ignoré, ce qui
/// permet d'y laisser des notes ou des sources.
const AUDIO_EXTENSIONS: [&str; 3] = ["mp3", "ogg", "wav"];

/// Silence entre deux morceaux, en secondes. Un enchaînement immédiat donne
/// l'impression d'un seul morceau incohérent.
const GAP: f32 = 1.5;

/// La musique de manche joue plus bas que celle des menus.
///
/// C'est par les bruitages que passe l'information pendant une partie — un
/// signe reconnu, une vie perdue. Une musique à plein volume les couvrirait au
/// moment précis où ils comptent.
const GAME_LEVEL: f32 = 0.6;

/// Quelle ambiance l'écran courant demande.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ambience {
    Menus,
    Game,
}

/// Une liste de morceaux et son ordre de passage.
struct Playlist {
    /// Les fichiers repérés, encore encodés : les décoder tous saturerait la
    /// mémoire pour rien.
    tracks: Vec<&'static File<'static>>,
    /// L'ordre de passage, remélangé à chaque tour.
    order: Vec<usize>,
    /// Position dans `order`.
    next: usize,
    /// Dernier morceau joué, pour ne pas le redonner en tête du tour suivant.
    last: Option<usize>,
}

impl Playlist {
    fn new(directory: &'static Dir<'static>) -> Self {
        let mut tracks: Vec<_> = directory
            .files()
            .filter(|file| {
                file.path()
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .map(|extension| AUDIO_EXTENSIONS.contains(&extension.to_lowercase().as_str()))
                    .unwrap_or(false)
            })
            .collect();

        // `files()` ne garantit pas d'ordre : on trie pour que le mélange parte
        // toujours de la même liste, et donc soit reproductible à graine égale.
        tracks.sort_by_key(|file| file.path());

        let mut playlist = Self { tracks, order: Vec::new(), next: 0, last: None };
        playlist.reshuffle();
        playlist
    }

    fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    fn take_next(&mut self) -> Option<(usize, &'static File<'static>)> {
        if self.tracks.is_empty() {
            return None;
        }
        if self.next >= self.order.len() {
            self.reshuffle();
        }

        let index = *self.order.get(self.next)?;
        self.next += 1;
        self.last = Some(index);
        Some((index, self.tracks[index]))
    }

    /// Remélange l'ordre de passage.
    ///
    /// Le premier morceau du nouveau tour ne doit pas être celui qui vient de
    /// jouer : sans cette précaution, un tirage sur trois pistes redonnerait
    /// assez souvent deux fois la même d'affilée, ce qui s'entend comme un bug.
    fn reshuffle(&mut self) {
        self.order = (0..self.tracks.len()).collect();
        self.next = 0;

        // Mélange de Fisher-Yates.
        for position in (1..self.order.len()).rev() {
            let other = rand::gen_range(0, position + 1);
            self.order.swap(position, other);
        }

        if self.order.len() > 1 {
            if let Some(last) = self.last {
                if self.order[0] == last {
                    self.order.swap(0, 1);
                }
            }
        }
    }
}

pub struct Music {
    menus: Playlist,
    game: Playlist,
    /// L'ambiance en cours, une fois qu'un morceau a démarré.
    current: Option<Ambience>,
    playing: Option<Playing>,
    /// Décompte du silence entre deux morceaux.
    silence: f32,
    /// Volume voulu par le joueur, avant la baisse propre à la manche.
    volume: f32,
}

struct Playing {
    sound: Sound,
    /// Secondes restantes, connues parce que le décodage donne le nombre exact
    /// d'échantillons. Le moteur audio, lui, ne prévient pas de la fin.
    remaining: f32,
}

impl Music {
    pub fn load(volume: f32) -> Self {
        Self {
            menus: Playlist::new(&MENU_MUSIC),
            game: Playlist::new(&GAME_MUSIC),
            current: None,
            playing: None,
            silence: 0.0,
            volume,
        }
    }

    /// Fait vivre la playlist. À appeler une fois par frame.
    pub async fn update(&mut self, dt: f32, ambience: Ambience) {
        if self.current != Some(ambience) {
            // Changement d'ambiance : on coupe net et on enchaîne sans attendre.
            // Laisser finir le morceau des menus par-dessus une manche déjà
            // lancée serait pire qu'une coupure franche.
            self.stop();
            self.current = Some(ambience);
            self.silence = 0.0;
        }

        if self.playlist(ambience).is_empty() {
            return;
        }

        if let Some(playing) = &mut self.playing {
            playing.remaining -= dt;
            if playing.remaining > 0.0 {
                return;
            }
            self.stop();
            self.silence = GAP;
        }

        self.silence -= dt;
        if self.silence <= 0.0 {
            self.start_next(ambience).await;
        }
    }

    /// Change le volume, y compris celui du morceau en cours : le réglage doit
    /// s'entendre pendant qu'on le bouge, pas au morceau suivant.
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);

        if let Some(playing) = &self.playing {
            set_sound_volume(&playing.sound, self.gain());
        }
    }

    pub fn stop(&mut self) {
        if let Some(playing) = self.playing.take() {
            stop_sound(&playing.sound);
        }
    }

    /// Le volume réellement appliqué, baisse de manche comprise.
    fn gain(&self) -> f32 {
        match self.current {
            Some(Ambience::Game) => self.volume * GAME_LEVEL,
            _ => self.volume,
        }
    }

    fn playlist(&self, ambience: Ambience) -> &Playlist {
        match ambience {
            Ambience::Menus => &self.menus,
            Ambience::Game => &self.game,
        }
    }

    async fn start_next(&mut self, ambience: Ambience) {
        let playlist = match ambience {
            Ambience::Menus => &mut self.menus,
            Ambience::Game => &mut self.game,
        };
        let Some((_, file)) = playlist.take_next() else { return };

        let Some(Decoded { wav, seconds }) = decode(file) else {
            // Un fichier illisible est sauté plutôt que réessayé en boucle.
            return;
        };

        if let Ok(sound) = load_sound_from_bytes(&wav).await {
            macroquad::audio::play_sound(
                &sound,
                PlaySoundParams { looped: false, volume: self.gain() },
            );
            self.playing = Some(Playing { sound, remaining: seconds });
        }
    }
}

struct Decoded {
    /// Le morceau réencodé en WAV, prêt pour le moteur audio.
    wav: Vec<u8>,
    /// Sa durée, connue exactement grâce au nombre d'échantillons décodés.
    /// Le moteur audio, lui, ne prévient pas de la fin d'un son.
    seconds: f32,
}

/// Décode un fichier audio et le réemballe en WAV.
///
/// L'écriture se fait au fil du décodage plutôt qu'en accumulant d'abord tout
/// le morceau en flottants : sur une piste de plusieurs minutes, cela évite de
/// tenir deux copies complètes en mémoire au même instant.
///
/// Renvoie `None` si le fichier est illisible.
fn decode(file: &'static File<'static>) -> Option<Decoded> {
    let stream = MediaSourceStream::new(Box::new(Cursor::new(file.contents())), Default::default());

    // L'extension oriente la détection du format ; symphonia vérifie ensuite le
    // contenu réel, un fichier mal nommé est donc quand même reconnu.
    let mut hint = Hint::new();
    if let Some(extension) = file.path().extension().and_then(|value| value.to_str()) {
        hint.with_extension(extension);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, stream, &FormatOptions::default(), &MetadataOptions::default())
        .ok()?;
    let mut format = probed.format;

    let track = format.default_track()?;
    let track_id = track.id;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .ok()?;

    // Les 44 premiers octets sont réservés à l'en-tête, qui ne peut être écrit
    // qu'à la fin : il annonce la taille des données.
    let mut wav = vec![0; WAV_HEADER_SIZE];
    let mut rate = 0;
    let mut channels = 0;

    while let Ok(packet) = format.next_packet() {
        if packet.track_id() != track_id {
            continue;
        }

        // Un paquet abîmé n'invalide pas le reste du morceau.
        let Ok(decoded) = decoder.decode(&packet) else { continue };

        let spec = *decoded.spec();
        rate = spec.rate;
        channels = spec.channels.count() as u16;

        let mut buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
        buffer.copy_interleaved_ref(decoded);
        for sample in buffer.samples() {
            wav.extend_from_slice(&to_pcm16(*sample).to_le_bytes());
        }
    }

    if wav.len() <= WAV_HEADER_SIZE || rate == 0 || channels == 0 {
        return None;
    }

    let frames = (wav.len() - WAV_HEADER_SIZE) / 2 / channels as usize;
    write_wav_header(&mut wav, rate, channels);

    Some(Decoded { wav, seconds: frames as f32 / rate as f32 })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn playlists() -> [Playlist; 2] {
        [Playlist::new(&MENU_MUSIC), Playlist::new(&GAME_MUSIC)]
    }

    #[test]
    fn only_audio_files_are_picked_up() {
        // Les dossiers contiennent une notice en Markdown, qui ne doit pas se
        // retrouver dans la playlist.
        for playlist in playlists() {
            for track in &playlist.tracks {
                let extension =
                    track.path().extension().and_then(|value| value.to_str()).unwrap_or("");
                assert!(
                    AUDIO_EXTENSIONS.contains(&extension.to_lowercase().as_str()),
                    "« {} » n'est pas un fichier audio",
                    track.path().display()
                );
            }
        }
    }

    #[test]
    fn every_shipped_track_can_be_decoded() {
        // Un fichier exotique ou tronque serait silencieusement saute en jeu :
        // mieux vaut le savoir en lancant les tests.
        for playlist in playlists() {
            for track in &playlist.tracks {
                let decoded = decode(track);
                assert!(decoded.is_some(), "« {} » est illisible", track.path().display());
                assert!(
                    decoded.unwrap().seconds > 0.5,
                    "« {} » ne dure presque rien : le decodage a echoue en cours de route",
                    track.path().display()
                );
            }
        }
    }

    #[test]
    fn the_order_covers_every_track_exactly_once() {
        for mut playlist in playlists() {
            if playlist.tracks.len() < 2 {
                continue;
            }
            playlist.reshuffle();

            let mut seen = playlist.order.clone();
            seen.sort_unstable();
            assert_eq!(seen, (0..playlist.tracks.len()).collect::<Vec<_>>());
        }
    }

    #[test]
    fn a_new_round_never_opens_on_the_track_that_just_played() {
        // Sur une poignee de pistes, le hasard redonnerait assez souvent deux
        // fois la meme d'affilee, ce qui s'entend comme un bug.
        let mut playlist = Playlist::new(&GAME_MUSIC);
        if playlist.tracks.len() < 2 {
            return;
        }

        for _ in 0..200 {
            let previous = playlist.last;
            let (index, _) = playlist.take_next().expect("liste non vide");
            if playlist.next == 1 {
                assert_ne!(Some(index), previous, "deux fois le meme morceau de suite");
            }
        }
    }

    #[test]
    fn an_empty_folder_is_not_an_error() {
        // Le jeu doit se lancer avant que la moindre musique n'ait ete ajoutee.
        let mut playlist = Playlist::new(&GAME_MUSIC);
        playlist.tracks.clear();
        playlist.reshuffle();

        assert!(playlist.is_empty());
        assert!(playlist.take_next().is_none());
    }

    #[test]
    fn the_round_plays_quieter_than_the_menus() {
        // Les bruitages portent l'information pendant une partie : une musique
        // a plein volume les couvrirait au moment ou ils comptent.
        let mut music = Music::load(1.0);

        music.current = Some(Ambience::Menus);
        let menus = music.gain();
        music.current = Some(Ambience::Game);

        assert!(music.gain() < menus);
    }
}
