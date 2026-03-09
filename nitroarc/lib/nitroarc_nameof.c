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

#include <assert.h>
#include <stddef.h>
#include <stdint.h>

#include "nitroarc_private.h"

#define skip_d() p_enttab += 3 + nlen; continue
#define skip_f() p_enttab += 1 + nlen; continue

static int validate_nameof_input(const nitroarc_t *narc, uint16_t i) {
    if (!narc->named)                 return NITROARC_ESUCCESS;
    if ((i & FNTB_FILESYS_MASKTYPE))  return NITROARC_EINDEXDIR;
    if (narc->nfiles <= i)            return NITROARC_EINDEXRANGE;
    return NITROARC_ESUCCESS;
}

static size_t nameof_size_unchecked(const nitroarc_t *narc, uint16_t i) {
    const char *p_fntb   = narc->fntb.data;
    const char *p_dirtab = p_fntb;
    const char *p_enttab = p_fntb + leu32(p_dirtab);
    const char *p_subent = NULL;
    uint16_t    i_firstf = 0;
    size_t      size     = 1;

loopback:
    while (*p_enttab != 0) {
        const uint8_t mask = *p_enttab;
        const uint8_t type = mask & FNTB_DIRENT_MASKTYPE;
        const uint8_t nlen = mask & FNTB_DIRENT_MASKSIZE;

        if (type == FNTB_DIRENT_TYPEFILE) {
            if (i_firstf < i) { i_firstf++; skip_f(); }
            return size + nlen;
        }

        uint16_t    dirid = leu16(&p_enttab[1 + nlen]) & FNTB_FILESYS_MASKID;
        const char *p_ent = p_fntb + (FNTB_DIRTAB_ENTRYSIZE * dirid);
        uint16_t    first = leu16(&p_ent[4]);
        if (first <= i) p_subent = p_enttab;

        skip_d();
    }

    const uint8_t nlen = *p_subent & FNTB_DIRENT_MASKSIZE;
    size += nlen + 1;

    uint16_t dirid = leu16(&p_subent[1 + nlen]) & FNTB_FILESYS_MASKID;
    p_dirtab       = p_fntb + (FNTB_DIRTAB_ENTRYSIZE * dirid);
    p_enttab       = p_fntb + leu32(p_dirtab);
    i_firstf       = leu16(&p_dirtab[4]);
    goto loopback;
}

int nitroarc_nameof_size(const nitroarc_t *narc, uint16_t i, size_t *out_size) {
    assert(narc);
    assert(out_size);

    int errc = validate_nameof_input(narc, i);
    if (errc) return errc;

    *out_size = narc->named ? nameof_size_unchecked(narc, i) : 1;
    return NITROARC_ESUCCESS;
}

int nitroarc_nameof(const nitroarc_t *narc, uint16_t i, char *buf, size_t size) {
    assert(narc);
    assert(buf);
    assert(size);

    if (!narc->named) {
        *buf = 0;
        return NITROARC_ESUCCESS;
    }

    int errc = validate_nameof_input(narc, i);
    if (errc) return errc;

    char       *p_buf    = buf;
    const char *p_fntb   = narc->fntb.data;
    const char *p_dirtab = p_fntb;
    const char *p_enttab = p_fntb + leu32(p_dirtab);
    const char *p_subent = NULL;
    uint16_t    i_firstf = 0;

loopback:
    while (*p_enttab != 0) {
        const uint8_t mask = *p_enttab;
        const uint8_t type = mask & FNTB_DIRENT_MASKTYPE;
        const uint8_t nlen = mask & FNTB_DIRENT_MASKSIZE;

        if (type == FNTB_DIRENT_TYPEFILE) {
            if (i_firstf < i) { i_firstf++; skip_f(); }

            for (uint8_t j = 0; j < nlen && size > 1; j++, size--) {
                *p_buf++ = *++p_enttab;
            }
            *p_buf = 0;
            return NITROARC_ESUCCESS;
        }

        // If this directory says it has a file with ID less than or equal to
        // the requested file, then the pointer to its subdirectory entry.
        uint16_t    dirid = leu16(&p_enttab[1 + nlen]) & FNTB_FILESYS_MASKID;
        const char *p_ent = p_fntb + (FNTB_DIRTAB_ENTRYSIZE * dirid);
        uint16_t    first = leu16(&p_ent[4]);
        if (first <= i) p_subent = p_enttab;

        skip_d();
    }


    // The last-recorded subdirectory is the next destination; copy its name,
    // then continue.
    if (p_subent == NULL) return NITROARC_ENULL;
    const uint8_t nlen = *p_subent & FNTB_DIRENT_MASKSIZE;
    for (uint8_t j = 0; j < nlen && size > 1; j++, size--) {
        *p_buf++ = *++p_subent;
    }
    *p_buf++ = '/';

    uint16_t dirid = leu16(++p_subent) & FNTB_FILESYS_MASKID;
    p_dirtab       = p_fntb + (FNTB_DIRTAB_ENTRYSIZE * dirid);
    p_enttab       = p_fntb + leu32(p_dirtab);
    i_firstf       = leu16(&p_dirtab[4]);
    goto loopback;
}

#undef skip_f
#undef skip_d
