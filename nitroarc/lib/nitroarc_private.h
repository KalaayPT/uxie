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

#ifndef NITROARC_PRIVATE_H
#define NITROARC_PRIVATE_H

#include <stddef.h>
#include <stdint.h>

#define SIGNATURE_NARC 0x4352414E // "NARC" in little-endian
#define SIGNATURE_BTAF 0x46415442 // "BTAF" in little-endian
#define SIGNATURE_BTNF 0x464E5442 // "BTNF" in little-endian
#define SIGNATURE_GMIF 0x46494D47 // "GMIF" in little-endian

#define BOM_BIGEND 0xFEFF
#define BOM_LITEND 0xFFFE

#define NARC_VERSION    0x0100
#define NARC_HEADERSIZE 0x10
#define NARC_SECTIONS   0x03

#define NARC_MAXFILES   0xF000

#define FATB_HEADERSIZE 0x0C
#define FNTB_HEADERSIZE 0x08
#define FIMG_HEADERSIZE 0x08

#define FATB_ENTRYSIZE (sizeof(uint32_t) * 2)

#define FNTB_DIRTAB_ENTRYSIZE (sizeof(uint32_t) + (2 * sizeof(uint16_t)))
#define FNTB_FILESYS_MASKID   0x0FFF
#define FNTB_FILESYS_MASKTYPE 0xF000

#define FNTB_DIRENT_MASKSIZE 0x7F
#define FNTB_DIRENT_MASKTYPE 0x80
#define FNTB_DIRENT_TYPEFILE 0x00
#define FNTB_DIRENT_TYPEDIR  0x80

#define PACKCTL_ZERO      0
#define PACKCTL_NAMED    (1 << 0)
#define PACKCTL_STRIPPED (1 << 1)

#define PACKCTL_ISNAMED(p)    (((p)->flags & PACKCTL_NAMED) != 0)
#define PACKCTL_ISSTRIPPED(p) (((p)->flags & PACKCTL_STRIPPED) != 0)

struct nitroarc_file {
    char    *name;
    uint32_t offset;
    uint32_t size;
};

static inline uint16_t leu16(const void *_s) {
    const uint8_t *s = _s;
    return (uint16_t)((s[1] << 8) | (s[0]));
}

static inline uint32_t leu32(const void *_s) {
    const uint8_t *s = _s;
    return (s[3] << 24) | (s[2] << 16) | (s[1] << 8) | s[0];
}

static inline void putleu16(void *_s, uint16_t u) {
    uint8_t *s = _s;
    s[0] = ((unsigned)u >> 0) & 0xFF;
    s[1] = ((unsigned)u >> 8) & 0xFF;
}

static inline void putleu32(void *_s, uint32_t u) {
    uint8_t *s = _s;
    s[0] = (uint8_t)((u >> 0) & 0xFF);
    s[1] = (uint8_t)((u >> 8) & 0xFF);
    s[2] = (uint8_t)((u >> 16) & 0xFF);
    s[3] = (uint8_t)((u >> 24) & 0xFF);
}

#define align_u(u)   (-(u) & 3)
#define align_u32(u) (-(u) & 31)
#define align_d(d)   (-(uintptr_t)(d) & 3)

static inline ptrdiff_t ptrdiff(const void *_l, const void *_r) {
    const unsigned char *l = _l;
    const unsigned char *r = _r;

    return l - r;
}

static inline char* strchrnul(const char *_s, int _c) {
    const unsigned char  c = (unsigned char)_c;
    const unsigned char *s = (void *)_s;

    while (*s && *s != c) s++;
    return (char *)s;
}

#endif // NITROARC_PRIVATE_H
