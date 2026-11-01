/* SPDX-License-Identifier: MIT
 * vm_main.c — plays a patch on a computer, for the tests: reads an .alb bank
 * (or an .alp patch), plays its first patch as one note, and writes the
 * samples (int32 Q16, little-endian) to the standard output.
 *
 *   vm_main PATCH freq_hz velocity hold_seconds total_seconds [level]
 */
#include <stdio.h>
#include <stdlib.h>
#include "../alea_vm.h"

int main(int argc, char **argv)
{
    static uint8_t buf[1 << 20];
    static int16_t pool[1 << 18];
    static int32_t words[ALEA_MAX_OPS + ALEA_MAX_STATES * 12];
    static int32_t out[256];
    alea_vm_t vm;
    alea_voice_t v;
    FILE *f;
    size_t n;
    const uint8_t *patch;
    uint32_t size, total, hold, i, err;
    double freq, vel, level = 1.0;
    if (argc < 6) { fprintf(stderr, "usage: vm_main PATCH freq vel hold total [level]\n"); return 2; }
    f = fopen(argv[1], "rb");
    if (!f) { perror(argv[1]); return 2; }
    n = fread(buf, 1, sizeof buf, f);
    fclose(f);
    if (n >= 9 && memcmp(buf, "ALB1", 4) == 0) {   /* a bank: its first patch */
        size = alea_rd32(buf + 21);
        patch = buf + 25;
    } else { patch = buf; size = (uint32_t)n; }
    err = alea_vm_load(&vm, patch, size);
    if (err) { fprintf(stderr, "alea_vm_load: %u\n", err); return 1; }
    freq = atof(argv[2]); vel = atof(argv[3]);
    hold = (uint32_t)(atof(argv[4]) * ALEA_FS); total = (uint32_t)(atof(argv[5]) * ALEA_FS);
    if (argc > 6) level = atof(argv[6]);
    alea_voice_init(&vm, &v, words, pool, sizeof pool / sizeof pool[0]);
    alea_voice_on(&vm, &v, (int32_t)(freq * 65536), (int32_t)(vel * 65536), (int32_t)(level * 65536));
    fprintf(stderr, "ops %u states %u words %u pool %u release %.3f struck %.3f\n", vm.nops, vm.nstates, vm.words, vm.pool,
            vm.release_q16 / 65536.0, vm.struck_q16 / 65536.0);
    for (i = 0; i < total; ) {
        uint32_t m = total - i < 256 ? total - i : 256, k;
        if (v.gate && i >= hold) alea_voice_gate(&v, 0);
        if (v.gate && i + m > hold) m = hold - i;          /* up to the release, exactly */
        for (k = 0; k < m; k++) out[k] = 0;
        alea_voice_render(&vm, &v, out, m);
        fwrite(out, sizeof out[0], m, stdout);
        i += m;
    }
    fprintf(stderr, "silent: %d\n", alea_voice_silent(&vm, &v));
    return 0;
}
