/* SPDX-License-Identifier: GPL-3.0-only
 * host_main.c — a mock of Felucca's host, enough to compile eng_alea.c against
 * felucca_engine.h on a computer and play a bank's first patch: the test
 * `fm1::tests::the_engine_builds_for_felucca`. Prints the bank, then the
 * note's loudness and when the voice ended. */
#include <stdint.h>
#include <string.h>
#include <stdio.h>
#include <math.h>
#include <stdlib.h>
#define NVOICE 8
#define NPART 4
#define FS 44100u
#define CTL 32
enum { P_LEVEL, P_ATK, P_DEC, P_SUS, P_REL, P_PAN, P_FTYPE, P_E0, P_E1, P_E2, P_E3, P_E4, P_E5, P_E6, P_E7, P_COUNT };
#include "felucca_engine.h"
typedef struct track { int16_t p[P_COUNT]; voice_t v[NVOICE]; } track_t;
static track_t trk[NPART];
/* 440 Hz at A4, the host's increment: the real table is in Felucca */
static inline uint32_t pitch_inc(uint32_t p) { return (uint32_t)(440.0 * pow(2.0, (p / 16.0 - 69) / 12) / FS * 4294967296.0); }
static inline int32_t amp_at(const vmod_t *m, uint32_t i) { (void)i; return m->amp0; }
static inline int32_t mulq15(int32_t a, int32_t b) { return (a * b) >> 15; }
#define VOICE_FS 24000
static inline int32_t voice_amp(int32_t s, const vmod_t *m, uint32_t i) { return mulq15(mulq15(s, amp_at(m, i)), VOICE_FS); }
#define ALEA_NO_SECTIONS
#include "../eng_alea.c"

static uint8_t bank[1 << 16];
static uint32_t bank_n;
const uint8_t *alea_bank_bytes(void) { return bank; }
uint32_t alea_bank_size(void) { return bank_n; }

int main(int argc, char **argv)
{
    FILE *f;
    track_t *t = &trk[0];
    voice_t *v = &t->v[0];
    vmod_t m;
    int32_t out[CTL];
    uint32_t k, blocks, hold;
    double e = 0, peak = 0;
    if (argc < 4) { fprintf(stderr, "usage: host_main BANK hold_s total_s\n"); return 2; }
    f = fopen(argv[1], "rb");
    if (!f) { perror(argv[1]); return 2; }
    bank_n = (uint32_t)fread(bank, 1, sizeof bank, f);
    fclose(f);
    alea_bank_boot();
    printf("bank: %u patches, first `%s` ok=%d\n", alea_bank.n, alea_bank.name[0], alea_bank.ok[0]);
    if (!alea_bank.n || !alea_bank.ok[0]) return 1;
    memset(&m, 0, sizeof m);
    m.amp0 = m.amp1 = 32767;
    t->p[P_E0] = 0;
    v->pitch16 = 69 * 16; v->vel = 100; v->gate = 1;
    hold = (uint32_t)(atof(argv[2]) * FS / CTL);
    blocks = (uint32_t)(atof(argv[3]) * FS / CTL);
    ENG_ALEA.note_on(t, v);
    for (k = 0; k < blocks; k++) {
        uint32_t i;
        if (k == hold) v->gate = 0;
        memset(out, 0, sizeof out);
        ENG_ALEA.render(t, v, out, CTL, &m);
        for (i = 0; i < CTL; i++) { e += (double)out[i] * out[i]; if (fabs((double)out[i]) > peak) peak = fabs((double)out[i]); }
        if (ENG_ALEA.done(t, v)) { printf("done at %.2f s\n", k * CTL / (double)FS); break; }
    }
    printf("rms %.0f peak %.0f (Q15)\n", sqrt(e / ((k ? k : 1) * CTL)), peak);
    (void)alea_sysex_unpack;
    return 0;
}
