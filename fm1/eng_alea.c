/* SPDX-License-Identifier: GPL-3.0-only
 * eng_alea.c — the ALEA engine for Felucca (M-VAVE FM-1): plays patches
 * written in alea (`synth { … }`, compiled by `alea work.alea --fm1`), from a
 * bank of up to sixteen kept in flash.
 *
 * This file joins Felucca's engines (firmware/src/eng_*.c): it is included in
 * the host's translation unit after felucca_engine.h, like the others, and
 * registered in engines.c ENGINES[] as &ENG_ALEA. What it needs from the host:
 *   - alea_bank_bytes() / alea_bank_size(): the bank in flash (fm1/FORMAT.md,
 *     "ALB1"), written by the SysEx handler (F0 7D 'A' 'L' 'E' 'A' 01 … F7,
 *     seven bits to eight: see alea_sysex_unpack below);
 *   - a pool for the voices' delay and chorus lines: ALEA_POOL_WORDS int16 a
 *     voice (24 K: 48 KB a voice, 384 KB for eight — too much for the FM-1's
 *     pool next to everything else, so the default is 12 K: a delay of up to
 *     0.27 s a voice; longer delays fall back to dry, and the compiler says so:
 *     use Felucca's DELAY send for long echoes).
 *
 * EDIT 1: PATCH (which of the bank's patches), LEVL. The other EDIT values are
 * free for a later version (macros a patch could expose). The voice's
 * amplitude comes from the patch itself (its envelopes), so ownenv = 1: the
 * host's ADSR is not applied, LFO -> AMP and the fades still are, and a voice
 * ends when the patch has died away (alea_voice_silent).
 *
 * Budget: an op is a few integer multiplies; a patch of 30 ops costs about as
 * much as the FM6 engine's six operators. Eight voices of a 30-op patch at
 * 44.1 kHz fit the audio interrupt with room to spare; the CPU figure in
 * MENU > SYSTEM > INFO shows what a patch takes. */

#ifndef ALEA_POOL_WORDS
#define ALEA_POOL_WORDS (12 * 1024)
#endif

#define ALEA_FS FS
#include "alea_vm.h"

/* ---------------------------------------------------------------- bank --- */
/* the host gives the bank's bytes (flash, "ALB1" …) */
const uint8_t *alea_bank_bytes(void);
uint32_t alea_bank_size(void);

static struct {
    alea_vm_t vm[16];            /* the patches, loaded */
    char name[16][16];
    uint8_t n;                   /* patches in the bank */
    uint8_t ok[16];              /* 1: loaded */
} alea_bank;

/* the bank from flash into alea_bank: at boot and after a SysEx write */
static void alea_bank_boot(void)
{
    const uint8_t *b = alea_bank_bytes();
    uint32_t n = alea_bank_size(), p = 5, k;
    memset(&alea_bank, 0, sizeof alea_bank);
    if (!b || n < 5 || memcmp(b, "ALB1", 4) != 0) return;
    alea_bank.n = b[4] > 16 ? 16 : b[4];
    for (k = 0; k < alea_bank.n; k++) {
        uint32_t size;
        if (p + 20 > n) break;
        memcpy(alea_bank.name[k], b + p, 15);
        size = alea_rd32(b + p + 16);
        p += 20;
        if (p + size > n) break;
        alea_bank.ok[k] = alea_vm_load(&alea_bank.vm[k], b + p, size) == 0;
        p += size;
    }
}

/* a SysEx body (between F0 7D 'A' 'L' 'E' 'A' 01 and F7) unpacked, seven bits to eight: the bank's
 * bytes, into out (returns how many; 0 when out is too small). The host writes them to flash. */
static uint32_t alea_sysex_unpack(const uint8_t *in, uint32_t n, uint8_t *out, uint32_t cap)
{
    uint32_t i = 0, o = 0;
    while (i < n) {
        uint8_t high = in[i++];
        uint32_t k;
        for (k = 0; k < 7 && i < n; k++, i++) {
            if (o >= cap) return 0;
            out[o++] = (uint8_t)(in[i] | ((high >> k) & 1 ? 0x80 : 0));
        }
    }
    return o;
}

/* -------------------------------------------------------------- voices --- */
#ifdef ALEA_NO_SECTIONS
#define ALEA_POOL
#else
#define ALEA_POOL __attribute__((section(".pool")))
#endif
static alea_voice_t alea_voices[NPART][NVOICE] ALEA_POOL;
static int16_t alea_lines[NPART][NVOICE][ALEA_POOL_WORDS] ALEA_POOL;

static const alea_vm_t *alea_patch(const track_t *t)
{
    uint32_t k = (uint32_t)t->p[P_E0];
    if (k >= alea_bank.n || !alea_bank.ok[k]) return 0;
    return &alea_bank.vm[k];
}

static alea_voice_t *alea_voice(track_t *t, voice_t *v)
{
    uint32_t part = (uint32_t)(t - trk), slot = (uint32_t)(v - t->v);
    return &alea_voices[part % NPART][slot % NVOICE];
}

static void alea_note_on(track_t *t, voice_t *v)
{
    const alea_vm_t *vm = alea_patch(t);
    alea_voice_t *av = alea_voice(t, v);
    uint32_t part = (uint32_t)(t - trk) % NPART, slot = (uint32_t)(v - t->v) % NVOICE;
    int32_t freq_q16, vel_q16;
    if (!vm) return;
    if (!av->pool) alea_voice_init(vm, av, alea_lines[part][slot], ALEA_POOL_WORDS);
    /* the note's frequency, Hz Q16, from its pitch (1/16 semitone) through the host's phase-increment
     * table: inc = f / fs * 2^32, so f = inc * fs / 2^32 */
    {
        uint32_t inc = pitch_inc((uint32_t)(v->pitch16 < 0 ? 0 : v->pitch16 > 2047 ? 2047 : v->pitch16));
        int64_t f = ((int64_t)inc * FS) >> 16;                 /* Hz, Q16 */
        freq_q16 = (int32_t)(f > (20000 << ALEA_Q) ? (20000 << ALEA_Q) : f);
    }
    vel_q16 = ((int32_t)v->vel << ALEA_Q) / 127;
    alea_voice_on(vm, av, freq_q16, vel_q16, ALEA_ONE);
}

/* the voice's samples: the patch's output (±8, Q16) to Q15 at VOICE_FS, through the host's amplitude
 * ramp (LFO -> AMP, the fades; the ADSR is the patch's own) */
static void alea_render(track_t *t, voice_t *v, int32_t *out, uint32_t n, const vmod_t *m)
{
    static int32_t buf[CTL];
    const alea_vm_t *vm = alea_patch(t);
    alea_voice_t *av = alea_voice(t, v);
    uint32_t i;
    if (!vm || !av->active) return;
    if (av->gate && !v->gate) alea_voice_gate(av, 0);
    if (!av->gate && v->gate) alea_voice_gate(av, 1);
    memset(buf, 0, n * sizeof buf[0]);
    alea_voice_render(vm, av, buf, n);
    for (i = 0; i < n; i++) {
        int32_t s = buf[i] >> 1;                              /* Q16 ±8 -> Q15 ±4: a patch's 1.0 is full scale */
        s = s > 32767 ? 32767 : s < -32767 ? -32767 : s;
        out[i] += voice_amp(s, m, i) * 2;
    }
}

static int alea_done(track_t *t, voice_t *v)
{
    const alea_vm_t *vm = alea_patch(t);
    alea_voice_t *av = alea_voice(t, v);
    if (!vm) return 1;
    if (alea_voice_silent(vm, av)) { av->active = 0; return 1; }
    return 0;
}

/* the patch names, for EDIT 1 */
static const char *alea_patch_names[17];
static const param_desc_t *alea_desc(const track_t *t, uint32_t k)
{
    static param_desc_t d;
    uint32_t i;
    (void)t;
    if (k != 0) return 0;
    for (i = 0; i < 16; i++) alea_patch_names[i] = i < alea_bank.n ? alea_bank.name[i] : "-";
    alea_patch_names[16] = 0;
    d = (param_desc_t){"PATCH", F_ENUM, 0, 15, 0, alea_patch_names, 0};
    return &d;
}

static const preset_t ALEA_PRESETS[] = {
    {"ALEA 1", {0, 100, 0, 0, 0, 0, 0, 0}, {0, 40, 127, 40}, 0, 0, FX(0, 0, 0, 0)},
    {"ALEA 2", {1, 100, 0, 0, 0, 0, 0, 0}, {0, 40, 127, 40}, 0, 0, FX(0, 0, 0, 0)},
    {"ALEA 3", {2, 100, 0, 0, 0, 0, 0, 0}, {0, 40, 127, 40}, 0, 0, FX(0, 0, 0, 0)},
    {"ALEA 4", {3, 100, 0, 0, 0, 0, 0, 0}, {0, 40, 127, 40}, 0, 0, FX(0, 0, 0, 0)},
};

static const engine_t ENG_ALEA = {
    .name = "ALEA",
    .page_title = {"PATCH", "-"},
    .edit = {
        {"PATCH", F_INT, 0, 15, 0, 0, 0},
        {"LEVL", F_PCT, 0, 127, 100, 0, 0},
        {"-", F_INT, 0, 0, 0, 0, 0}, {"-", F_INT, 0, 0, 0, 0, 0}, {"-", F_INT, 0, 0, 0, 0, 0},
        {"-", F_INT, 0, 0, 0, 0, 0}, {"-", F_INT, 0, 0, 0, 0, 0}, {"-", F_INT, 0, 0, 0, 0, 0},
    },
    .presets = ALEA_PRESETS,
    .npresets = NELEM(ALEA_PRESETS),
    .note_on = alea_note_on,
    .render = alea_render,
    .knob = {P_E0, P_E1, P_LEVEL, P_PAN},
    .keep = 0,
    .desc = alea_desc,
    .ownenv = 1,
    .done = alea_done,
};
