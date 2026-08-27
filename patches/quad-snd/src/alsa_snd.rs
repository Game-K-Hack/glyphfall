// roughly based on http://equalarea.com/paul/alsa-audio.html

use crate::{error::Error, PlaySoundParams};

use quad_alsa_sys as sys;

use std::sync::mpsc;

pub use crate::mixer::Playback;

mod consts {
    /// Les périphériques essayés, dans l'ordre.
    ///
    /// L'original s'arrêtait à « default » et « pipewire », ce qui laisse muet
    /// un bureau pourtant sonore : quand PipeWire tient la carte, « default »
    /// retombe sur dmix, qui ne peut plus ouvrir un matériel déjà pris, et le
    /// périphérique « pipewire » n'existe que si le paquet du pont ALSA est
    /// installé — ce qu'aucune distribution ne garantit.
    ///
    /// « pulse » comble ce trou : son greffon vient d'un paquet répandu, et le
    /// serveur PulseAudio de PipeWire l'accepte. Les deux derniers sont des
    /// filets : le périphérique par défaut sans mixage, puis la première carte
    /// en accès direct.
    pub const DEVICES: &[&str] = &[
        "default\0",
        "pipewire\0",
        "pulse\0",
        "sysdefault\0",
        "hw:0\0",
    ];
    pub const RATE: u32 = 44100;
    pub const CHANNELS: u32 = 2;
    pub const PCM_BUFFER_SIZE: ::std::os::raw::c_ulong = 4096;
}

/// Ouvre le périphérique et lui impose nos réglages.
///
/// Rend `None` plutôt que de paniquer : une machine sans son n'est pas une
/// erreur de programmation, et une panique ici tuait le fil audio — après quoi
/// chaque son demandé imprimait « Audio thread died », à l'infini.
unsafe fn setup_pcm_device() -> Option<*mut sys::snd_pcm_t> {
    let mut pcm_handle = std::ptr::null_mut();

    if !consts::DEVICES.iter().any(|device| {
        sys::snd_pcm_open(
            &mut pcm_handle,
            device.as_ptr() as _,
            sys::SND_PCM_STREAM_PLAYBACK,
            0,
        ) >= 0
    }) {
        eprintln!("audio : aucun peripherique ALSA n'a pu etre ouvert");
        return None;
    }

    let mut hw_params: *mut sys::snd_pcm_hw_params_t = std::ptr::null_mut();
    sys::snd_pcm_hw_params_malloc(&mut hw_params);
    sys::snd_pcm_hw_params_any(pcm_handle, hw_params);

    if sys::snd_pcm_hw_params_set_access(pcm_handle, hw_params, sys::SND_PCM_ACCESS_RW_INTERLEAVED)
        < 0
    {
        eprintln!("audio : mode entrelace refuse");
        return None;
    }
    if sys::snd_pcm_hw_params_set_format(pcm_handle, hw_params, sys::SND_PCM_FORMAT_FLOAT_LE) < 0 {
        eprintln!("audio : format flottant refuse");
        return None;
    }
    if sys::snd_pcm_hw_params_set_channels(pcm_handle, hw_params, consts::CHANNELS) < 0 {
        eprintln!("audio : stereo refusee");
        return None;
    }

    let mut rate = consts::RATE;
    if sys::snd_pcm_hw_params_set_rate_near(pcm_handle, hw_params, &mut rate, std::ptr::null_mut())
        < 0
    {
        eprintln!("audio : frequence refusee");
        return None;
    }

    // La taille de tampon vient en dernier, et se négocie.
    //
    // L'original l'imposait à 4096 images exactement, et avant même d'avoir
    // fixé le nombre de voies et la fréquence. Chaque réglage passait, mais
    // leur écriture groupée échouait : une fois la stéréo et le 44,1 kHz
    // arrêtés, plus aucune configuration ne tenait dans un tampon de cette
    // taille exacte. D'où « Can't set harware parameters. » sur des machines
    // parfaitement saines. La variante « near » laisse ALSA choisir la valeur
    // possible la plus proche.
    let mut buffer_size = consts::PCM_BUFFER_SIZE;
    if sys::snd_pcm_hw_params_set_buffer_size_near(pcm_handle, hw_params, &mut buffer_size) < 0 {
        eprintln!("audio : taille de tampon refusee");
        return None;
    }

    if sys::snd_pcm_hw_params(pcm_handle, hw_params) < 0 {
        eprintln!("audio : reglages refuses par le peripherique");
        return None;
    }
    sys::snd_pcm_hw_params_free(hw_params);

    // tell ALSA to wake us up whenever AudioContext::PCM_BUFFER_SIZE or more frames
    //   of playback data can be delivered. Also, tell
    //   ALSA that we'll start the device ourselves.
    let mut sw_params: *mut sys::snd_pcm_sw_params_t = std::ptr::null_mut();

    if sys::snd_pcm_sw_params_malloc(&mut sw_params) < 0 {
        eprintln!("audio : cannot allocate software parameters structure");
        return None;
    }
    if sys::snd_pcm_sw_params_current(pcm_handle, sw_params) < 0 {
        eprintln!("audio : cannot initialize software parameters structure");
        return None;
    }

    // if sys::snd_pcm_sw_params_set_avail_min(
    //     pcm_handle,
    //     sw_params,
    //     AudioContext::PCM_BUFFER_SIZE,
    // ) < 0
    // {
    //     panic!("cannot set minimum available count");
    // }
    if sys::snd_pcm_sw_params_set_start_threshold(pcm_handle, sw_params, 0) < 0 {
        eprintln!("audio : cannot set start mode");
        return None;
    }
    if sys::snd_pcm_sw_params(pcm_handle, sw_params) < 0 {
        eprintln!("audio : cannot set software parameters");
        return None;
    }
    sys::snd_pcm_sw_params_free(sw_params);

    if sys::snd_pcm_prepare(pcm_handle) < 0 {
        eprintln!("audio : cannot prepare audio interface for use");
        return None;
    }

    Some(pcm_handle)
}

unsafe fn audio_thread(mut mixer: crate::mixer::Mixer) {
    let mut buffer: Vec<f32> = vec![0.0; consts::PCM_BUFFER_SIZE as usize * 2];

    let Some(pcm_handle) = setup_pcm_device() else {
        return silence(mixer);
    };

    loop {
        // Wait for PCM to be ready for next write (no timeout)
        if sys::snd_pcm_wait(pcm_handle, -1) < 0 {
            eprintln!("audio : le peripherique ne repond plus");
            return silence(mixer);
        }

        // // find out how much space is available for playback data
        // teoretically it should reduce latency - we will fill a minimum amount of
        // frames just to keep alsa busy and will be able to mix some fresh sounds
        // it does, but also randmly panics sometimes

        // let frames_to_deliver = sys::snd_pcm_avail_update(pcm_handle);
        // println!("{}", frames_to_deliver);
        // let frames_to_deliver = if frames_to_deliver > consts::PCM_BUFFER_SIZE as _ {
        //     consts::PCM_BUFFER_SIZE as i64
        // } else {
        //     frames_to_deliver
        // };

        let frames_to_deliver = consts::PCM_BUFFER_SIZE as i64;

        // ask mixer to fill the buffer
        mixer.fill_audio_buffer(&mut buffer, frames_to_deliver as usize);

        // send filled buffer back to alsa
        let frames_writen = sys::snd_pcm_writei(
            pcm_handle,
            buffer.as_ptr() as *const _,
            frames_to_deliver as _,
        );
        if frames_writen == -libc::EPIPE as ::std::os::raw::c_long {
            println!("Underrun occured: -EPIPE, attempting recover");

            sys::snd_pcm_recover(pcm_handle, frames_writen as _, 0);
        }

        if frames_writen > 0 && frames_writen != frames_to_deliver as _ {
            println!("Underrun occured: frames_writen != frames_to_deliver, attempting recover");

            sys::snd_pcm_recover(pcm_handle, frames_writen as _, 0);
        }
    }
}

/// Le fil audio quand il n'y a pas de son : il vide la file sans rien jouer.
///
/// Sans cela, le fil s'arrêterait, le canal se fermerait, et chaque son
/// demandé imprimerait « Audio thread died » — une ligne par bruitage, par
/// voix et par morceau, jusqu'à noyer la console. Le jeu doit pouvoir tourner
/// muet sans se plaindre à chaque frame.
fn silence(mut mixer: crate::mixer::Mixer) {
    let mut buffer: Vec<f32> = vec![0.0; consts::PCM_BUFFER_SIZE as usize * 2];

    loop {
        // Le mélangeur lit ses messages en remplissant le tampon : on le fait
        // tourner à peu près au rythme réel, puis on jette le résultat.
        mixer.fill_audio_buffer(&mut buffer, consts::PCM_BUFFER_SIZE as usize);
        std::thread::sleep(std::time::Duration::from_millis(
            consts::PCM_BUFFER_SIZE as u64 * 1000 / consts::RATE as u64,
        ));
    }
}

pub struct AudioContext {
    pub(crate) mixer_ctrl: crate::mixer::MixerControl,
}

impl AudioContext {
    pub fn new() -> AudioContext {
        use crate::mixer::Mixer;

        let (mixer_builder, mixer_ctrl) = Mixer::new();
        std::thread::spawn(move || unsafe {
            audio_thread(mixer_builder.build());
        });

        AudioContext { mixer_ctrl }
    }
}

pub struct Sound {
    sound_id: u32,
}

impl Sound {
    pub fn load(ctx: &AudioContext, data: &[u8]) -> Sound {
        let sound_id = ctx.mixer_ctrl.load(data);

        Sound { sound_id }
    }

    pub fn play(&self, ctx: &AudioContext, params: PlaySoundParams) -> Playback {
        ctx.mixer_ctrl.play(self.sound_id, params)
    }

    pub fn stop(&self, ctx: &AudioContext) {
        ctx.mixer_ctrl.stop_all(self.sound_id);
    }

    pub fn set_volume(&self, ctx: &AudioContext, volume: f32) {
        ctx.mixer_ctrl.set_volume_all(self.sound_id, volume);
    }

    pub fn delete(&self, ctx: &AudioContext) {
        ctx.mixer_ctrl.delete(self.sound_id);
    }
}
