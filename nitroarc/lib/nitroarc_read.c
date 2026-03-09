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

// Minimum allowable size for a standard NARC file: no member files
// +12 is the minimum size of an FNTB data section:
//   - 8 bytes for the root directory entry
//   - 1 byte for the subdirectory entry terminator
//   - 3 bytes of alignment padding
#define NARC_MINSIZE ( \
    NARC_HEADERSIZE    \
    + FATB_HEADERSIZE  \
    + FNTB_HEADERSIZE  \
    + FIMG_HEADERSIZE  \
    + 12               \
)

static int nitroarc_fatb(nitroarc_t *narc, int64_t *running_size);
static int nitroarc_fntb(nitroarc_t *narc, int64_t *running_size);
static int nitroarc_fimg(nitroarc_t *narc, int64_t *running_size);

static int read_stripped(const uint8_t *blob, uint32_t size, nitroarc_t *narc);

int nitroarc_read(const void *blob, uint32_t size, nitroarc_t *narc) {
    if (narc == NULL)        return NITROARC_ENULL;
    if (blob == NULL)        return NITROARC_ENULL;
    if (size < NARC_MINSIZE) return NITROARC_ESIZERANGE;
    *narc = (nitroarc_t){ 0 };

    const uint8_t *data = blob;
    if (leu32(&data[0x00]) != SIGNATURE_NARC)  return read_stripped(data, size, narc);
    if (leu16(&data[0x04]) != BOM_LITEND)      return NITROARC_EBYTEORDER;
    if (leu16(&data[0x06]) != NARC_VERSION)    return NITROARC_EVERSION;
    if (leu32(&data[0x08]) != size)            return NITROARC_EARCHIVESIZE;
    if (leu16(&data[0x0C]) != NARC_HEADERSIZE) return NITROARC_EHEADERSIZE;
    if (leu16(&data[0x0E]) != NARC_SECTIONS)   return NITROARC_ESECTIONS;

    narc->size      = (uint32_t)size;
    narc->head      = data;
    narc->fatb.data = data + NARC_HEADERSIZE; // Start of section

    // Safety: make sure the section sizes are sensible along the way
    int64_t running_size = size - NARC_HEADERSIZE;
    int errc = nitroarc_fatb(narc, &running_size)
            || nitroarc_fntb(narc, &running_size)
            || nitroarc_fimg(narc, &running_size)
            || NITROARC_ESUCCESS;

    if (errc) *narc = (nitroarc_t){ 0 };
    return errc;
}

static int nitroarc_fatb(nitroarc_t *narc, int64_t *running_size) {
    const uint8_t *head = narc->fatb.data;

    if (leu32(&head[0x00]) != SIGNATURE_BTAF) return NITROARC_EFATBSIG;

    uint32_t size = leu32(&head[0x04]);
    *running_size -= size;
    if (*running_size < 0) return NITROARC_EFATBSIZE;

    uint16_t nfiles = leu16(&head[0x08]);
    uint32_t xsize  = (uint32_t)(FATB_HEADERSIZE + (nfiles * FATB_ENTRYSIZE));
    if (size != xsize) return NITROARC_ENUMFILES;

    narc->fatb.size     = size;
    narc->fatb.data     = head + FATB_HEADERSIZE; // First entry pointer
    narc->fntb.data     = head + narc->fatb.size; // Head of next section
    narc->fatb.ofs_head = (uint32_t)ptrdiff(head, narc->head);
    narc->fatb.ofs_data = (uint32_t)ptrdiff(narc->fatb.data, narc->head);

    narc->nfiles = nfiles;
    return NITROARC_ESUCCESS;
}

static int nitroarc_fntb(nitroarc_t *narc, int64_t *running_size) {
    const uint8_t *head = narc->fntb.data;

    if (leu32(&head[0x00]) != SIGNATURE_BTNF) return NITROARC_EFNTBSIG;

    uint32_t size = leu32(&head[0x04]);
    *running_size -= size;
    if (*running_size < 0) return NITROARC_EFNTBSIZE;

    narc->named         = size > FNTB_HEADERSIZE + FNTB_DIRTAB_ENTRYSIZE;
    narc->fntb.size     = size;
    narc->fntb.data     = head + FNTB_HEADERSIZE;  // First entry pointer
    narc->fimg.data     = head + narc->fntb.size; // Head of next section
    narc->fntb.ofs_head = (uint32_t)ptrdiff(head, narc->head);
    narc->fntb.ofs_data = (uint32_t)ptrdiff(narc->fntb.data, narc->head);

    narc->ndirs = (uint16_t)(leu16(&((uint8_t *)narc->fntb.data)[0x06]) & 0xFFF);
    return NITROARC_ESUCCESS;
}

static int nitroarc_fimg(nitroarc_t *narc, int64_t *running_size) {
    const char *head = narc->fimg.data;
    if (leu32(&head[0x00]) != SIGNATURE_GMIF) return NITROARC_EFIMGSIG;

    uint32_t size = leu32(&head[0x04]);
    *running_size -= size;
    if (*running_size < 0) return NITROARC_EFIMGSIZE;

    narc->fimg.size = size;
    narc->fimg.data = head + FIMG_HEADERSIZE; // First entry pointer
    narc->fimg.ofs_head = (uint32_t)ptrdiff(head, narc->head);
    narc->fimg.ofs_data = (uint32_t)ptrdiff(narc->fimg.data, narc->head);

    return NITROARC_ESUCCESS;
}

static int read_stripped(const uint8_t *data, uint32_t size, nitroarc_t *narc) {
    if (size < 0x10) return NITROARC_ESIGNATURE;

    const uint32_t fntb_offs = leu32(&data[0x00]);
    const uint32_t fatb_offs = leu32(&data[0x08]);
    const uint32_t fntb_size = leu32(&data[0x04]);
    const uint32_t fatb_size = leu32(&data[0x0C]);

    uint32_t expected = fntb_offs + fntb_size + align_u32(fntb_offs + fntb_size);
    if (fntb_offs != 0x10 || fatb_offs != expected) {
        return NITROARC_ESIGNATURE;
    }

    narc->size = size;
    narc->head = data;

    narc->fatb.size = fatb_size;
    narc->fatb.data = data + fatb_offs;
    narc->fatb.ofs_head = 0x08;
    narc->fatb.ofs_data = fatb_offs;
    narc->nfiles = (uint16_t)(fatb_size / FATB_ENTRYSIZE);

    narc->named     = fntb_size == FNTB_DIRTAB_ENTRYSIZE;
    narc->fntb.size = fntb_size;
    narc->fntb.data = data + fntb_offs;
    narc->fntb.ofs_head = 0x00;
    narc->fntb.ofs_data = fntb_offs;
    narc->ndirs = (uint16_t)(leu16(&((uint8_t *)narc->fntb.data)[0x06]) & 0xFFF);

    narc->fimg.size = size - fntb_size - fatb_size - 0x10;
    narc->fimg.data = data; // all FATB offsets in stripped variant are absolute
    narc->fimg.ofs_head = 0x00;
    narc->fimg.ofs_data = 0x00;

    return NITROARC_ESUCCESS;
}
