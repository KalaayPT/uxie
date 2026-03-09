/*
 * #include <nitroarc_ffi.h> - FFI wrapper for Nintendo's Nitro Archive format
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

#include "nitroarc_ffi.h"

#include <stdlib.h>
#include <string.h>

#include "../lib/nitroarc_private.h"

struct nitroarcffi_archive {
    uint8_t    *stream;
    uint32_t    size;
    nitroarc_t  narc;
};

struct nitroarcffi_builder {
    nitroarc_packer_t packer;
    char            **names;
    uint16_t          n_names;
    uint16_t          c_names;
    uint16_t          n_expected;
    unsigned          named  : 1;
    unsigned          sealed : 1;
};

static void* ffi_malloc(void *ctx, unsigned items, unsigned size) {
    (void)ctx;

    if (size != 0U && (size_t)items > (SIZE_MAX / (size_t)size)) return NULL;
    return malloc((size_t)items * (size_t)size);
}

static void* ffi_realloc(void *ctx, void *ptr, unsigned items, unsigned size) {
    (void)ctx;

    if (size != 0U && (size_t)items > (SIZE_MAX / (size_t)size)) return NULL;
    return realloc(ptr, (size_t)items * (size_t)size);
}

static void ffi_free(void *ctx, void *ptr, unsigned items, unsigned size) {
    (void)ctx;
    (void)items;
    (void)size;

    free(ptr);
}



static int archive_open_owned(
    uint8_t *stream,
    uint32_t size,
    nitroarcffi_archive_t **out_archive
) {
    if (stream == NULL)      return NITROARC_ENULL;
    if (out_archive == NULL) return NITROARC_ENULL;

    nitroarcffi_archive_t *archive = malloc(sizeof(*archive));
    if (archive == NULL) {
        free(stream);
        return NITROARC_EALLOCFAIL;
    }

    int errc = nitroarc_read(stream, size, &archive->narc);
    if (errc) {
        free(stream);
        free(archive);
        return errc;
    }

    archive->stream = stream;
    archive->size   = size;

    *out_archive = archive;
    return NITROARC_ESUCCESS;
}

static char* clone_string(const char *s) {
    if (s == NULL) return NULL;

    size_t size = strlen(s) + 1;
    char  *tmp  = malloc(size);
    if (tmp == NULL) return NULL;

    memcpy(tmp, s, size);
    return tmp;
}

static void free_builder_names(nitroarcffi_builder_t *builder) {
    if (builder == NULL || builder->names == NULL) return;

    for (uint16_t i = 0; i < builder->n_names; i++) {
        free(builder->names[i]);
    }

    free(builder->names);
    builder->names   = NULL;
    builder->n_names = 0;
    builder->c_names = 0;
}

static void seal_builder(nitroarcffi_builder_t *builder) {
    builder->sealed               = 1;
    builder->packer.fimg_data     = NULL;
    builder->packer.fimg_size     = 0;
    builder->packer.fimg_capacity = 0;
    builder->packer.f_records     = NULL;
    builder->packer.f_count       = 0;
    builder->packer.f_capacity    = 0;
}

const char* nitroarcffi_errs(int errc) {
    switch (errc) {
    case NITROARCFFI_ESTATE: return "operation cannot be performed in current state";
    case NITROARCFFI_ECOUNT: return "invalid member count for this operation";
    default:                 return nitroarc_errs(errc);
    }
}

int nitroarcffi_archive_open(const void *stream, uint32_t size, nitroarcffi_archive_t **out_archive) {
    if (stream == NULL)      return NITROARC_ENULL;
    if (out_archive == NULL) return NITROARC_ENULL;
    if (size == 0)           return NITROARC_ESIZERANGE;

    *out_archive = NULL;

    uint8_t *copy = malloc((size_t)size);
    if (copy == NULL) return NITROARC_EALLOCFAIL;
    memcpy(copy, stream, size);

    return archive_open_owned(copy, size, out_archive);
}

void nitroarcffi_archive_close(nitroarcffi_archive_t *archive) {
    if (archive == NULL) return;

    free(archive->stream);
    free(archive);
}

int nitroarcffi_archive_count(const nitroarcffi_archive_t *archive, uint16_t *out_count) {
    if (archive == NULL)   return NITROARC_ENULL;
    if (out_count == NULL) return NITROARC_ENULL;

    *out_count = archive->narc.nfiles;
    return NITROARC_ESUCCESS;
}

int nitroarcffi_archive_geti(
    const nitroarcffi_archive_t *archive,
    uint16_t index,
    const void **out_member,
    uint32_t *out_size
) {
    if (archive == NULL)    return NITROARC_ENULL;
    if (out_member == NULL) return NITROARC_ENULL;
    if (out_size == NULL)   return NITROARC_ENULL;

    void    *data = NULL;
    uint32_t size = 0;
    int errc = nitroarc_geti(&archive->narc, index, &data, &size);
    if (errc) return errc;

    *out_member = data;
    *out_size   = size;
    return NITROARC_ESUCCESS;
}

int nitroarcffi_archive_gets(
    const nitroarcffi_archive_t *archive,
    const char *path,
    const void **out_member,
    uint32_t *out_size
) {
    if (archive == NULL)    return NITROARC_ENULL;
    if (path == NULL)       return NITROARC_ENULL;
    if (out_member == NULL) return NITROARC_ENULL;
    if (out_size == NULL)   return NITROARC_ENULL;

    void    *data = NULL;
    uint32_t size = 0;

    int errc = NITROARC_ESUCCESS;
    if (path[0] == '/') {
        errc = nitroarc_gets(&archive->narc, path, &data, &size);
    } else {
        size_t plen = strlen(path);
        char *abspath = malloc(plen + 2);
        if (abspath == NULL) return NITROARC_EALLOCFAIL;

        abspath[0] = '/';
        memcpy(&abspath[1], path, plen + 1);

        errc = nitroarc_gets(&archive->narc, abspath, &data, &size);
        free(abspath);
    }

    if (errc) return errc;

    *out_member = data;
    *out_size   = size;
    return NITROARC_ESUCCESS;
}

int nitroarcffi_archive_nameof(
    const nitroarcffi_archive_t *archive,
    uint16_t index,
    char *buf,
    size_t size
) {
    if (archive == NULL) return NITROARC_ENULL;
    if (buf == NULL)     return NITROARC_ENULL;
    if (size == 0)       return NITROARC_ESIZERANGE;

    return nitroarc_nameof(&archive->narc, index, buf, size);
}

int nitroarcffi_archive_nameof_alloc(
    const nitroarcffi_archive_t *archive,
    uint16_t index,
    char **out_name
) {
    if (archive == NULL)  return NITROARC_ENULL;
    if (out_name == NULL) return NITROARC_ENULL;

    *out_name = NULL;

    size_t size = 0;
    int errc = nitroarc_nameof_size(&archive->narc, index, &size);
    if (errc) return errc;

    char *name = malloc(size);
    if (name == NULL) return NITROARC_EALLOCFAIL;

    errc = nitroarc_nameof(&archive->narc, index, name, size);
    if (errc) {
        free(name);
        return errc;
    }

    *out_name = name;
    return NITROARC_ESUCCESS;
}

int nitroarcffi_builder_open(
    uint16_t nfiles,
    unsigned named,
    unsigned stripped,
    nitroarcffi_builder_t **out_builder
) {
    if (out_builder == NULL) return NITROARC_ENULL;
    *out_builder = NULL;

    if (nfiles == 0) return NITROARCFFI_ECOUNT;

    nitroarcffi_builder_t *builder = calloc(1, sizeof(*builder));
    if (builder == NULL) return NITROARC_EALLOCFAIL;

    if (named) {
        builder->names = calloc(nfiles, sizeof(*builder->names));
        if (builder->names == NULL) {
            free(builder);
            return NITROARC_EALLOCFAIL;
        }
        builder->c_names = nfiles;
    }
    builder->n_expected = nfiles;
    builder->named      = named != 0;

    builder->packer.ctx     = NULL;
    builder->packer.malloc  = ffi_malloc;
    builder->packer.realloc = ffi_realloc;
    builder->packer.free    = ffi_free;

    int errc = nitroarc_pinit(&builder->packer, nfiles, named, stripped);
    if (errc) {
        free_builder_names(builder);
        free(builder);
        return errc;
    }

    *out_builder = builder;
    return NITROARC_ESUCCESS;
}

int nitroarcffi_builder_ppack(
    nitroarcffi_builder_t *builder,
    const void *data,
    uint32_t size,
    const char *name
) {
    if (builder == NULL) return NITROARC_ENULL;
    if (builder->sealed) return NITROARCFFI_ESTATE;

    if (builder->packer.f_count >= builder->packer.f_capacity) {
        return NITROARCFFI_ECOUNT;
    }

    if (builder->named && name == NULL) {
        return NITROARC_EINVALIDPATH;
    }

    if (builder->named && builder->n_names >= builder->c_names) {
        return NITROARCFFI_ESTATE;
    }

    if (!builder->named) name = NULL;

    char *name_owned = clone_string(name);
    if (builder->named && name_owned == NULL) return NITROARC_EALLOCFAIL;

    int errc = nitroarc_ppack(&builder->packer, (void *)data, size, name_owned);
    if (errc) {
        free(name_owned);
        return errc;
    }

    if (builder->named) {
        builder->names[builder->n_names++] = name_owned;
    }

    return NITROARC_ESUCCESS;
}

int nitroarcffi_builder_pseal(
    nitroarcffi_builder_t *builder,
    void **out_data,
    uint32_t *out_size
) {
    if (builder == NULL)  return NITROARC_ENULL;
    if (out_data == NULL) return NITROARC_ENULL;
    if (out_size == NULL) return NITROARC_ENULL;
    if (builder->sealed)  return NITROARCFFI_ESTATE;

    *out_data = NULL;
    *out_size = 0;

    if (builder->packer.f_count != builder->n_expected) {
        return NITROARCFFI_ECOUNT;
    }

    int errc = nitroarc_pseal(&builder->packer, out_data, out_size);
    seal_builder(builder);
    free_builder_names(builder);
    return errc;
}

void nitroarcffi_builder_close(nitroarcffi_builder_t *builder) {
    if (builder == NULL) return;

    if (!builder->sealed) {
        builder->packer.free(
            builder->packer.ctx,
            builder->packer.fimg_data,
            builder->packer.fimg_size,
            1
        );
        builder->packer.free(
            builder->packer.ctx,
            builder->packer.f_records,
            builder->packer.f_count,
            sizeof(*builder->packer.f_records)
        );
    }

    free_builder_names(builder);
    free(builder);
}

void nitroarcffi_free(void *ptr) {
    free(ptr);
}
