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

int nitroarc_geti(const nitroarc_t *narc, uint16_t i, void **out_member, uint32_t *out_size) {
    if (narc == NULL)                return NITROARC_ENULL;
    if ((i & FNTB_FILESYS_MASKTYPE)) return NITROARC_EINDEXDIR;
    if (narc->nfiles <= i)           return NITROARC_EINDEXRANGE;

    if (out_member != NULL && out_size != NULL) {
        const uint8_t *p_fatb = (uint8_t *)narc->fatb.data;
        const uint8_t *p_fent = p_fatb + (i * FATB_ENTRYSIZE);
        const uint32_t ofsbeg = leu32(p_fent);
        const uint32_t ofsend = leu32(p_fent + sizeof(uint32_t));

        *out_member = (uint8_t *)narc->fimg.data + ofsbeg;
        *out_size   = ofsend - ofsbeg;
    }

    return NITROARC_ESUCCESS;
}
