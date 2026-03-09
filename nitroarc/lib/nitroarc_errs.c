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

static const char* errors[] = {
    [NITROARC_ESUCCESS]       = "success",
    [NITROARC_ENULL]          = "a required parameter is NULL",
    [NITROARC_ESIZERANGE]     = "input size is out-of-range",
    [NITROARC_ESIGNATURE]     = "archive has an invalid file signature",
    [NITROARC_EBYTEORDER]     = "archive has an invalid byte-order mark",
    [NITROARC_EVERSION]       = "archive has an unsupported version",
    [NITROARC_EARCHIVESIZE]   = "archive reports a different size from the input",
    [NITROARC_EHEADERSIZE]    = "archive has an invalid header size",
    [NITROARC_ESECTIONS]      = "archive has an invalid number of sections",
    [NITROARC_ENUMFILES]      = "archive member count does not align with its size",
    [NITROARC_EFATBSIG]       = "allocation table has an invalid section signature",
    [NITROARC_EFATBSIZE]      = "allocation table reports an invalid section size",
    [NITROARC_EFNTBSIG]       = "filename table has an invalid section signature",
    [NITROARC_EFNTBSIZE]      = "filename table reports an invalid section size",
    [NITROARC_EFIMGSIG]       = "file image table has an invalid section signature",
    [NITROARC_EFIMGSIZE]      = "file image table reports an invalid section size",
    [NITROARC_EINDEXDIR]      = "requested index would be a directory, not a file",
    [NITROARC_EINDEXRANGE]    = "requested index is greater than the number of members",
    [NITROARC_EINVALIDPATH]   = "requested filepath is improperly constructed",
    [NITROARC_ENOSUCHFILE]    = "no such member for the requested filepath",
    [NITROARC_EALLOCDEF]      = "allocation interface is improperly defined",
    [NITROARC_EALLOCFAIL]     = "allocation failure",
    [NITROARC_ETOOMANYFILES]  = "too many files expected",
    [NITROARC_ESIZEOVERFLOW]  = "output size exceeds 32-bit maximum",

    [NITROARC_EMAX] = "unknown error code",
};

const char* nitroarc_errs(int errc) {
    errc = errc >= NITROARC_ESUCCESS && errc < NITROARC_EMAX
         ? errc
         : NITROARC_EMAX;
    return errors[errc];
}
