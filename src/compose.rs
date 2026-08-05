//! Le générateur de « Claude », la musique d'ambiance des menus.
//!
//! C'est un programme de composition, pas un fichier audio : le morceau est
//! écrit ici en accords et en phrases, puis rendu en `.wav`. Le régénérer après
//! avoir changé une valeur prend une seconde, ce qui permet de le retoucher.
//!
//! Le parti pris est l'inverse de celui des bruitages. Les blips 8-bit veulent
//! des ondes carrées qui claquent ; une nappe qui tourne pendant trois minutes
//! derrière un écran de menu veut des sinusoïdes, des attaques lentes et de
//! l'écho. La synthèse additive donne d'excellents résultats sur ce registre,
//! là où elle peine à produire une mélodie chiptune convaincante.
//!
//! Lancer : `GLYPHFALL_COMPOSE=assets/music/menu/Claude.wav cargo run --release`

use crate::audio::{WAV_HEADER_SIZE, to_pcm16, write_wav_header};

/// 22 050 Hz suffit : tout le contenu du morceau est fait de sinusoïdes dont la
/// partielle la plus aiguë reste sous 4 kHz, loin de la limite de 11 kHz. Aller
/// plus haut doublerait le poids du fichier sans rien ajouter d'audible.
const SAMPLE_RATE: u32 = 22_050;

const BPM: f32 = 68.0;
const BEAT: f32 = 60.0 / BPM;
const BEATS_PER_BAR: f32 = 4.0;
const BAR: f32 = BEAT * BEATS_PER_BAR;

/// Un cycle harmonique complet : quatre accords de deux mesures.
const BARS_PER_CYCLE: f32 = 8.0;
/// Six cycles font quarante-huit mesures, soit deux minutes et quarante-neuf.
const CYCLES: usize = 6;

/// Fondu d'ouverture et de fermeture, en secondes. Le morceau doit s'installer
/// sans qu'on le remarque et se retirer de même.
const FADE_IN: f32 = 4.0;
const FADE_OUT: f32 = 8.0;

/// Crête visée après normalisation. On laisse de la marge sous 1.0 : le moteur
/// audio ajoute son propre volume par-dessus.
const PEAK: f32 = 0.82;

/// La grille d'accords, en numéros de note MIDI.
///
/// Do majeur septième, la mineur septième, fa majeur septième, sol sixte. Une
/// suite sans tension : aucun accord n'appelle de résolution, elle peut donc
/// tourner indéfiniment sans jamais donner l'impression de vouloir finir.
struct Chord {
    /// La fondamentale, tenue par la basse.
    root: f32,
    /// Les notes tenues par la nappe.
    pad: [f32; 3],
    /// Les notes égrenées par l'arpège.
    arp: [f32; 4],
}

const PROGRESSION: [Chord; 4] = [
    // Do majeur 7
    Chord { root: 36.0, pad: [64.0, 67.0, 71.0], arp: [60.0, 64.0, 67.0, 71.0] },
    // La mineur 7
    Chord { root: 33.0, pad: [60.0, 64.0, 67.0], arp: [57.0, 60.0, 64.0, 67.0] },
    // Fa majeur 7
    Chord { root: 29.0, pad: [60.0, 64.0, 65.0], arp: [53.0, 57.0, 60.0, 64.0] },
    // Sol sixte
    Chord { root: 31.0, pad: [59.0, 62.0, 64.0], arp: [55.0, 59.0, 62.0, 64.0] },
];

/// Une phrase mélodique : instant en temps depuis le début du cycle, note MIDI,
/// durée en temps.
type Phrase = [(f32, f32, f32)];

/// Phrase principale : des notes longues, une par accord ou presque, qui
/// laissent le silence respirer entre elles.
const MELODY_A: &Phrase = &[
    (0.0, 76.0, 3.0),
    (4.0, 74.0, 2.5),
    (8.0, 72.0, 3.0),
    (12.0, 69.0, 2.5),
    (16.0, 72.0, 3.0),
    (20.0, 76.0, 2.5),
    (24.0, 74.0, 4.0),
    (29.0, 71.0, 2.5),
];

/// Variation : monte d'une tierce et ajoute une note par mesure, sans jamais
/// devenir volubile.
const MELODY_B: &Phrase = &[
    (0.0, 79.0, 2.0),
    (2.5, 76.0, 1.5),
    (4.0, 81.0, 3.0),
    (8.0, 79.0, 2.0),
    (10.5, 76.0, 1.5),
    (12.0, 72.0, 3.0),
    (16.0, 77.0, 2.0),
    (18.5, 74.0, 1.5),
    (20.0, 81.0, 3.0),
    (24.0, 79.0, 2.0),
    (26.5, 74.0, 1.5),
    (28.0, 71.0, 4.0),
];

/// Fin : trois notes seulement, de plus en plus espacées.
const MELODY_OUTRO: &Phrase = &[(0.0, 72.0, 4.0), (12.0, 76.0, 4.0), (24.0, 69.0, 6.0)];

/// Ce que joue chaque cycle. Le morceau s'installe voix par voix, culmine au
/// quatrième cycle, puis se retire dans l'ordre inverse.
struct Section {
    bass: bool,
    melody: Option<&'static Phrase>,
    arpeggio: bool,
}

const SECTIONS: [Section; CYCLES] = [
    Section { bass: false, melody: None, arpeggio: false },
    Section { bass: true, melody: None, arpeggio: false },
    Section { bass: true, melody: Some(MELODY_A), arpeggio: false },
    Section { bass: true, melody: Some(MELODY_B), arpeggio: true },
    Section { bass: true, melody: Some(MELODY_A), arpeggio: true },
    Section { bass: true, melody: Some(MELODY_OUTRO), arpeggio: false },
];

/// Compose le morceau et l'écrit à l'emplacement demandé.
#[cfg(not(target_arch = "wasm32"))]
pub fn write(path: &str) {
    let samples = render();
    let seconds = samples.len() as f32 / SAMPLE_RATE as f32;

    let mut wav = vec![0; WAV_HEADER_SIZE];
    wav.reserve(samples.len() * 2);
    for sample in &samples {
        wav.extend_from_slice(&to_pcm16(*sample).to_le_bytes());
    }
    write_wav_header(&mut wav, SAMPLE_RATE, 1);

    match std::fs::write(path, wav) {
        Ok(()) => println!(
            "écrit : {path} — {}:{:02}",
            (seconds as u32) / 60,
            (seconds as u32) % 60
        ),
        Err(error) => eprintln!("échec sur {path} : {error}"),
    }
}

/// Rend le morceau entier en échantillons mono.
pub fn render() -> Vec<f32> {
    let total_seconds = CYCLES as f32 * BARS_PER_CYCLE * BAR + FADE_OUT;
    let length = (total_seconds * SAMPLE_RATE as f32) as usize;

    let mut bed = vec![0.0; length]; // nappe et basse
    let mut voices = vec![0.0; length]; // mélodie et arpège, qui passeront par l'écho

    for (index, section) in SECTIONS.iter().enumerate() {
        let cycle_start = index as f32 * BARS_PER_CYCLE * BAR;

        for (position, chord) in PROGRESSION.iter().enumerate() {
            // Chaque accord tient deux mesures.
            let start = cycle_start + position as f32 * 2.0 * BAR;

            for note in chord.pad {
                pad(&mut bed, start, 2.0 * BAR, note);
            }
            if section.bass {
                bass(&mut bed, start, 2.0 * BAR, chord.root);
            }
            if section.arpeggio {
                arpeggio(&mut voices, start, 2.0 * BAR, &chord.arp);
            }
        }

        if let Some(phrase) = section.melody {
            for (beat, note, length) in phrase {
                bell(&mut voices, cycle_start + beat * BEAT, length * BEAT, *note);
            }
        }
    }

    // L'écho est ce qui donne l'impression d'espace ; sans lui la mélodie sonne
    // sèche et posée sur la nappe plutôt que dedans.
    echo(&mut voices, BEAT * 1.5, 0.38);

    let mut mix: Vec<f32> =
        bed.iter().zip(&voices).map(|(bed, voice)| bed + voice * 0.9).collect();

    fade(&mut mix);
    normalize(&mut mix);
    mix
}

/// La nappe : deux oscillateurs légèrement désaccordés, attaque très lente.
///
/// Le désaccord fait battre les deux ondes l'une contre l'autre, ce qui produit
/// un mouvement lent et chaleureux qu'une sinusoïde seule n'a jamais.
fn pad(output: &mut [f32], start: f32, length: f32, midi: f32) {
    const ATTACK: f32 = 1.6;
    const RELEASE: f32 = 2.2;
    const DETUNE: f32 = 1.004;
    const GAIN: f32 = 0.16;

    let frequency = hertz(midi);

    render_note(output, start, length + RELEASE, |t| {
        let wave = sine(frequency, t)
            + sine(frequency * DETUNE, t) * 0.8
            + sine(frequency * 2.0, t) * 0.18
            + sine(frequency * 3.0, t) * 0.06;

        // Une respiration très lente, pour que la nappe ne soit jamais figée.
        let breath = 0.88 + 0.12 * sine(0.07, t);

        wave * envelope(t, length, ATTACK, RELEASE) * breath * GAIN
    });
}

/// La basse : une sinusoïde pure, presque sourde, qui ne fait que poser le sol.
fn bass(output: &mut [f32], start: f32, length: f32, midi: f32) {
    const ATTACK: f32 = 0.6;
    const RELEASE: f32 = 1.4;
    const GAIN: f32 = 0.34;

    let frequency = hertz(midi);

    render_note(output, start, length + RELEASE, |t| {
        let wave = sine(frequency, t) + sine(frequency * 2.0, t) * 0.12;
        wave * envelope(t, length, ATTACK, RELEASE) * GAIN
    });
}

/// La mélodie : un timbre de cloche douce, obtenu par une partielle légèrement
/// désaccordée de l'octave et une décroissance exponentielle.
fn bell(output: &mut [f32], start: f32, length: f32, midi: f32) {
    const GAIN: f32 = 0.30;
    /// Constante de temps de l'extinction. Plus elle est longue, plus la note
    /// traîne et se mêle à la suivante.
    const TAU: f32 = 1.5;

    let frequency = hertz(midi);
    let tail = length + 2.0;

    render_note(output, start, tail, |t| {
        let wave = sine(frequency, t)
            + sine(frequency * 2.01, t) * 0.30
            + sine(frequency * 3.02, t) * 0.10;

        // Attaque courte mais pas instantanée : c'est un maillet, pas un clic.
        let attack = (t / 0.02).min(1.0);
        wave * attack * (-t / TAU).exp() * GAIN
    });
}

/// L'arpège : les notes de l'accord égrenées en croches, très en retrait.
fn arpeggio(output: &mut [f32], start: f32, length: f32, notes: &[f32; 4]) {
    const GAIN: f32 = 0.10;
    const TAU: f32 = 0.5;

    let step = BEAT / 2.0;
    let count = (length / step) as usize;

    for index in 0..count {
        // Montée puis descente, plutôt qu'un cycle qui redémarre sèchement.
        let position = index % 6;
        let note = notes[if position < 4 { position } else { 6 - position }];
        let frequency = hertz(note);

        render_note(output, start + index as f32 * step, 1.2, |t| {
            let attack = (t / 0.01).min(1.0);
            sine(frequency, t) * attack * (-t / TAU).exp() * GAIN
        });
    }
}

/// Ajoute un écho à un signal, en le repliant sur lui-même avec atténuation.
fn echo(buffer: &mut [f32], delay_seconds: f32, feedback: f32) {
    let delay = (delay_seconds * SAMPLE_RATE as f32) as usize;
    if delay == 0 || delay >= buffer.len() {
        return;
    }

    // Un seul passage suffit : comme chaque répétition réalimente la suivante,
    // l'écho se prolonge tout seul en s'éteignant.
    for index in delay..buffer.len() {
        buffer[index] += buffer[index - delay] * feedback;
    }
}

/// Fondus d'ouverture et de fermeture sur le mixage complet.
fn fade(mix: &mut [f32]) {
    let length = mix.len() as f32;
    let fade_in = FADE_IN * SAMPLE_RATE as f32;
    let fade_out = FADE_OUT * SAMPLE_RATE as f32;

    for (index, sample) in mix.iter_mut().enumerate() {
        let position = index as f32;
        let opening = (position / fade_in).min(1.0);
        let closing = ((length - position) / fade_out).min(1.0);

        // Élever au carré donne un fondu plus naturel à l'oreille qu'une rampe
        // droite, qui semble s'ouvrir trop vite puis ralentir.
        *sample *= (opening * closing).powi(2);
    }
}

/// Ramène la crête du mixage au niveau visé.
///
/// Écrêter serait audible comme une distorsion : mieux vaut mesurer le maximum
/// réel et diviser, plutôt que de deviner des volumes qui tomberaient juste.
fn normalize(mix: &mut [f32]) {
    let peak = mix.iter().fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
    if peak <= f32::EPSILON {
        return;
    }

    let gain = PEAK / peak;
    for sample in mix.iter_mut() {
        *sample *= gain;
    }
}

/// Additionne une note dans le tampon, en appelant `voice` pour chaque
/// échantillon avec le temps écoulé depuis le début de la note.
fn render_note(output: &mut [f32], start: f32, length: f32, voice: impl Fn(f32) -> f32) {
    let first = (start * SAMPLE_RATE as f32).max(0.0) as usize;
    let count = (length * SAMPLE_RATE as f32) as usize;

    for index in 0..count {
        let Some(sample) = output.get_mut(first + index) else { break };
        *sample += voice(index as f32 / SAMPLE_RATE as f32);
    }
}

/// Enveloppe attaque / maintien / extinction, en cosinus surélevé.
fn envelope(t: f32, length: f32, attack: f32, release: f32) -> f32 {
    let opening = if t < attack {
        // Une rampe droite laisserait entendre le coude au sommet de l'attaque.
        0.5 - 0.5 * (std::f32::consts::PI * t / attack).cos()
    } else {
        1.0
    };

    let closing = if t > length {
        (1.0 - (t - length) / release).max(0.0)
    } else {
        1.0
    };

    opening * closing * closing
}

fn sine(frequency: f32, t: f32) -> f32 {
    (std::f32::consts::TAU * frequency * t).sin()
}

/// Le la du diapason porte le numéro 69 ; chaque demi-ton multiplie la
/// fréquence par la racine douzième de deux.
fn hertz(midi: f32) -> f32 {
    440.0 * 2.0_f32.powf((midi - 69.0) / 12.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_piece_lasts_between_two_and_a_half_and_three_minutes() {
        let seconds = render().len() as f32 / SAMPLE_RATE as f32;

        assert!(
            (150.0..=180.0).contains(&seconds),
            "durée obtenue : {}:{:02}",
            (seconds as u32) / 60,
            (seconds as u32) % 60
        );
    }

    #[test]
    fn the_mix_never_clips() {
        // Une somme de quatre voix dépasse largement 1.0 avant normalisation ;
        // l'écrêtage à l'encodage s'entendrait comme une saturation.
        let peak = render().iter().fold(0.0_f32, |peak, sample| peak.max(sample.abs()));

        assert!(peak <= 1.0, "crête à {peak}");
        assert!((peak - PEAK).abs() < 0.01, "la normalisation doit viser {PEAK}, obtenu {peak}");
    }

    #[test]
    fn the_piece_opens_and_closes_on_silence() {
        let samples = render();

        assert!(samples[0].abs() < 0.001, "le morceau doit s'ouvrir sur du silence");
        assert!(
            samples[samples.len() - 1].abs() < 0.001,
            "et se refermer de même, pour boucler sans claquement"
        );
    }

    #[test]
    fn every_voice_actually_sounds() {
        // Une erreur de placement rendrait une voix inaudible sans que rien
        // n'échoue : on vérifie que chacune produit du signal isolément.
        let length = (10.0 * SAMPLE_RATE as f32) as usize;
        let energy = |buffer: &[f32]| buffer.iter().map(|s| s.abs()).sum::<f32>();

        let mut buffer = vec![0.0; length];
        pad(&mut buffer, 0.0, 4.0, 64.0);
        assert!(energy(&buffer) > 1.0, "la nappe est muette");

        let mut buffer = vec![0.0; length];
        bass(&mut buffer, 0.0, 4.0, 36.0);
        assert!(energy(&buffer) > 1.0, "la basse est muette");

        let mut buffer = vec![0.0; length];
        bell(&mut buffer, 0.0, 2.0, 76.0);
        assert!(energy(&buffer) > 1.0, "la mélodie est muette");

        let mut buffer = vec![0.0; length];
        arpeggio(&mut buffer, 0.0, 4.0, &[60.0, 64.0, 67.0, 71.0]);
        assert!(energy(&buffer) > 1.0, "l'arpège est muet");
    }

    #[test]
    fn the_echo_repeats_after_the_delay() {
        let mut buffer = vec![0.0; SAMPLE_RATE as usize];
        buffer[0] = 1.0;

        echo(&mut buffer, 0.1, 0.5);

        let delay = (0.1 * SAMPLE_RATE as f32) as usize;
        assert_eq!(buffer[delay], 0.5, "première répétition");
        assert_eq!(buffer[delay * 2], 0.25, "la réinjection prolonge l'écho");
    }
}
