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

#ifndef NITROARC_DISABLE_PACKING_API

#include "nitroarc.h"

#include <assert.h>
#include <stddef.h>
#include <stdint.h>

#include "nitroarc_private.h"

int nitroarc_ppack(nitroarc_packer_t *p, void *data, uint32_t size, char *name) {
    assert(p);
    assert(p->malloc);
    assert(p->realloc);
    assert(p->free);

    if (!data || size == 0) {
        p->f_records[p->f_count] = (nitroarc_file_t){
            .name   = name,
            .offset = p->fimg_size,
            .size   = 0,
        };
        p->f_count++;
        return NITROARC_ESUCCESS;
    }

    uint32_t align = align_u(size);

    if ((size_t)(p->fimg_size + size + align) >= (size_t)p->fimg_capacity) {
        size_t cap = p->fimg_capacity;
        while (p->fimg_size + size + align > cap) cap *= 2;

        if (cap > UINT32_MAX) return NITROARC_ESIZEOVERFLOW;

        void *tmp = p->realloc(p->ctx, p->fimg_data, (uint32_t)cap, 1);
        if (tmp == NULL) return NITROARC_EALLOCFAIL;

        p->fimg_data     = tmp;
        p->fimg_capacity = (uint32_t)cap;
    }

    if (PACKCTL_ISNAMED(p)) {
        if (name == NULL) return NITROARC_EINVALIDPATH;

        for (const char *p = name; p && *p; p++) {
            switch (*p & 0xFF) {
            default: break;

            case '\\':
            case '?':
            case '"':
            case '<':
            case '>':
            case '*':
            case ':':
            case ';':
            case '|':
                return NITROARC_EINVALIDPATH;
            }
        }
    }

    uint8_t *fimg_src = data;
    uint8_t *fimg_dst = (uint8_t *)p->fimg_data + p->fimg_size;
    for (size_t i = 0; i < size; i++)  fimg_dst[i] = fimg_src[i];
    for (size_t i = 0; i < align; i++) fimg_dst[size + i] = 0xFF;

    p->f_records[p->f_count] = (nitroarc_file_t){
        .name   = name,
        .offset = p->fimg_size,
        .size   = size,
    };

    p->fimg_size += size + align;
    p->f_count++;

    return NITROARC_ESUCCESS;
}

#endif
