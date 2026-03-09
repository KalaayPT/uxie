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
#include <stdint.h>

#include "nitroarc_private.h"

int nitroarc_pinit(
    nitroarc_packer_t *p,
    uint16_t nfiles,
    unsigned named,
    unsigned stripped
) {
    assert(p);
    assert(p->malloc);
    assert(p->realloc);
    assert(p->free);
    assert(nfiles > 0);

    if (nfiles > NARC_MAXFILES) return NITROARC_ETOOMANYFILES;

    p->flags         = 0;
    p->fimg_capacity = nfiles * 1024;
    p->fimg_size     = 0;
    p->fimg_data     = p->malloc(p->ctx, p->fimg_capacity, 1);
    p->f_capacity    = nfiles;
    p->f_count       = 0;
    p->f_records     = p->malloc(p->ctx, nfiles, sizeof(*p->f_records));

    if (p->fimg_data == NULL) goto erralloc;
    if (p->f_records == NULL) goto erralloc;
    if (named)    p->flags |= PACKCTL_NAMED;
    if (stripped) p->flags |= PACKCTL_STRIPPED;

    return NITROARC_ESUCCESS;

erralloc:
    p->free(p->ctx, p->fimg_data, p->fimg_capacity, 1);
    p->free(p->ctx, p->f_records, nfiles, sizeof(*p->f_records));
    return NITROARC_EALLOCFAIL;
}

#endif
