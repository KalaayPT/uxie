/*
 * #include <nitroarc.h> - implementation of Nintendo's Nitro Archive format
 * Copyright (C) 2026 Rachel <rachel@lhea.me>
 *
 * This library is free software: you can redistribute it and/or modify it under
 * the terms of the GNU Lesser General Public License as published by the Free
 * Software Foundation, either version 3 of the License, or (at your option) any
 * any later version.
 *
 * This library is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 * FOR A PARTICULAR PURPOSE.  See the GNU Lesser General Public License for more
 * details.
 *
 * You should have received a copy of the GNU Lesser General Public License
 * along with this library.  If not, see <https://www.gnu.org/licenses/>.
 */

#include "nitroarc.h"

#include <stddef.h>
#include <stdint.h>

#include "nitroarc_private.h"

// Similar to `strncmp`, but returns either `1` (if the strings are equivalent
// across a prefix of size `n`) or `0` (if they aren't).
//
// More formally:
// - `strnequ(l, r, n) == 1` is equivalent to `strncmp(l, r, n) == 0`
// - `strnequ(l, r, n) == 0` is equivalent to `strncmp(l, r, n) != 0`
static int strnequ(const char *_l, const char *_r, size_t n) {
    const unsigned char *l = (void *)_l;
    const unsigned char *r = (void *)_r;

    if (n == 0) return 1; // 0-length comparison is treated as "equal"

    for (n--; *l && *r && n && *l == *r; l++, r++, n--);
    return *l == *r;
}

#define skip_d() p_dirent += 3 + size; continue
#define skip_f() p_dirent += 1 + size; continue

int nitroarc_gets(const nitroarc_t *narc, const char *s, void **out_member, uint32_t *out_size) {
    if (narc == NULL) return NITROARC_ENULL;
    if (s == NULL)    return NITROARC_ENULL;
    if (s[0] != '/')  return NITROARC_EINVALIDPATH;
    if (s[1] == '\0') return NITROARC_EINVALIDPATH; // Only search for leaves

    const char *p_path   = &s[1];
    const char *p_dirsep = strchrnul(p_path, '/');
    size_t      compsize = p_dirsep - p_path;

    const char *p_fntb   = narc->fntb.data;
    const char *p_dirtab = p_fntb;
    const char *p_dirent = p_fntb + leu32(p_dirtab);

    uint16_t id_found = 0xFFFF;
    uint16_t id       = 0;
    while (*p_dirent != 0 && id_found == 0xFFFF && id < FNTB_FILESYS_MASKTYPE) {
        const uint8_t mask = *p_dirent;
        const uint8_t type = mask & FNTB_DIRENT_MASKTYPE;
        const uint8_t size = mask & FNTB_DIRENT_MASKSIZE;

        if (*p_dirsep == '/') {
            if (type != FNTB_DIRENT_TYPEDIR)             { skip_f(); }
            if (strnequ(p_path, p_dirent + 1, compsize)) { skip_d(); }

            if (p_dirent[1] == '\0') return NITROARC_EINVALIDPATH;

            uint16_t id_subdir = leu16(p_dirent + 1 + compsize);

            p_dirtab = p_fntb + (id_subdir & FNTB_FILESYS_MASKID);
            p_dirent = p_fntb + leu32(p_dirtab);
            p_path   = p_dirsep + 1;
            p_dirsep = strchrnul(p_path, '/');
            compsize = p_dirsep - p_path;
            id       = leu16(p_dirtab + 4);

            continue;
        }

        if (type != FNTB_DIRENT_TYPEFILE)            {       skip_d(); }
        if (strnequ(p_path, p_dirent + 1, compsize)) { id++; skip_f(); }

        id_found = id;
    }

    return *p_dirent != 0
        ? nitroarc_geti(narc, id_found, out_member, out_size)
        : NITROARC_ENOSUCHFILE;
}

#undef skip_d
#undef skip_f
