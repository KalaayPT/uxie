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

#ifndef NITROARC_FFI_H
#define NITROARC_FFI_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stddef.h>
#include <stdint.h>

#include "nitroarc.h"

#if defined(_WIN32)
#  if defined(NITROARC_FFI_BUILD_SHARED)
#    define NITROARC_FFI_API __declspec(dllexport)
#  elif defined(NITROARC_FFI_USE_SHARED)
#    define NITROARC_FFI_API __declspec(dllimport)
#  else
#    define NITROARC_FFI_API
#  endif
#elif defined(__GNUC__)
#  define NITROARC_FFI_API __attribute__((visibility("default")))
#else
#  define NITROARC_FFI_API
#endif

typedef struct nitroarcffi_archive nitroarcffi_archive_t;
typedef struct nitroarcffi_builder nitroarcffi_builder_t;

enum {
    NITROARCFFI_ESTATE = NITROARC_EMAX + 1,
    NITROARCFFI_ECOUNT,
};

/*
 * Get a printable, user-facing error message for either a base-library or FFI
 * error.
 */
NITROARC_FFI_API const char* nitroarcffi_errs(int errc);

/*
 * Read an archive from an in-memory stream, and retain an owned copy of it.
 *
 * `stream` must be non-NULL and `size` must be greater than 0, otherwise
 * `NITROARC_ENULL` or `NITROARC_ESIZERANGE` is returned respectively.
 *
 * Any pointers returned by `nitroarcffi_archive_geti()` and
 * `nitroarcffi_archive_gets()` remain valid until `nitroarcffi_archive_close()`
 * is called on this handle.
 */
NITROARC_FFI_API int nitroarcffi_archive_open(
    const void *stream,
    uint32_t size,
    nitroarcffi_archive_t **out_archive
);

/*
 * Close an archive handle and release all resources owned by it.
 */
NITROARC_FFI_API void nitroarcffi_archive_close(nitroarcffi_archive_t *archive);

/*
 * Query the number of members in an opened archive handle.
 */
NITROARC_FFI_API int nitroarcffi_archive_count(
    const nitroarcffi_archive_t *archive,
    uint16_t *out_count
);

/*
 * Get the image data for a member by index.
 */
NITROARC_FFI_API int nitroarcffi_archive_geti(
    const nitroarcffi_archive_t *archive,
    uint16_t index,
    const void **out_member,
    uint32_t *out_size
);

/*
 * Get the image data for a member by virtual archive path.
 *
 * `path` is an archive-internal virtual path, not a host path. It uses `/` as
 * the directory separator and is expected to be UTF-8 encoded.
 *
 * This routine accepts either an absolute archive path beginning with `/` or a
 * relative archive path without that leading `/`.
 */
NITROARC_FFI_API int nitroarcffi_archive_gets(
    const nitroarcffi_archive_t *archive,
    const char *path,
    const void **out_member,
    uint32_t *out_size
);

/*
 * Get the virtual path for a member by index.
 *
 * Archive member paths are archive-internal virtual paths, not host paths.
 * They use `/` as the directory separator and are expected to be UTF-8
 * encoded.
 *
 * The returned path does not begin with a leading `/`.
 *
 * This routine writes at most `size` bytes including the zero-terminator.
 */
NITROARC_FFI_API int nitroarcffi_archive_nameof(
    const nitroarcffi_archive_t *archive,
    uint16_t index,
    char *buf,
    size_t size
);

/*
 * Allocate the virtual path for a member by index as a UTF-8 string.
 *
 * Archive member paths are archive-internal virtual paths, not host paths.
 * They use `/` as the directory separator and are expected to be UTF-8
 * encoded.
 *
 * The returned path does not begin with a leading `/`.
 *
 * On success, `*out_name` receives a zero-terminated string which must be
 * released with `nitroarcffi_free()`.
 */
NITROARC_FFI_API int nitroarcffi_archive_nameof_alloc(
    const nitroarcffi_archive_t *archive,
    uint16_t index,
    char **out_name
);

/*
 * Initialize an archive builder for `nfiles` members.
 *
 * `nfiles` must be greater than 0, otherwise `NITROARCFFI_ECOUNT` is returned.
 *
 * `nfiles` must also satisfy the base-library limit (61,440 members),
 * otherwise `NITROARC_ETOOMANYFILES` is returned.
 */
NITROARC_FFI_API int nitroarcffi_builder_open(
    uint16_t nfiles,
    unsigned named,
    unsigned stripped,
    nitroarcffi_builder_t **out_builder
);

/*
 * Add one member to an archive builder.
 *
 * If this builder was initialized with `named != 0`, `name` must be non-NULL.
 * If this builder was initialized with `named == 0`, `name` is ignored.
 *
 * Member names are archive-internal virtual paths, not host paths. They use
 * `/` as the directory separator and are expected to be UTF-8 encoded.
 */
NITROARC_FFI_API int nitroarcffi_builder_ppack(
    nitroarcffi_builder_t *builder,
    const void *data,
    uint32_t size,
    const char *name
);

/*
 * Seal a builder into an owned archive stream.
 *
 * The value used for `nfiles` when opening this builder is strict: this call
 * will return `NITROARCFFI_ECOUNT` unless exactly that many files were packed.
 *
 * If this call fails with `NITROARCFFI_ECOUNT`, the builder is unchanged and
 * remains usable (you may pack additional members and retry).
 *
 * Otherwise, this call consumes the builder regardless of success or failure.
 *
 * The returned `*out_data` buffer must be released with `nitroarcffi_free()`.
 */
NITROARC_FFI_API int nitroarcffi_builder_pseal(
    nitroarcffi_builder_t *builder,
    void **out_data,
    uint32_t *out_size
);

/*
 * Close a builder handle and release all resources owned by it.
 */
NITROARC_FFI_API void nitroarcffi_builder_close(nitroarcffi_builder_t *builder);

/*
 * Release a dynamic allocation returned by this FFI API.
 *
 * This must only be used on pointers returned by this FFI layer, such as the
 * buffer returned by `nitroarcffi_builder_pseal()` or the string returned by
 * `nitroarcffi_archive_nameof_alloc()`.
 */
NITROARC_FFI_API void nitroarcffi_free(void *ptr);

#ifdef __cplusplus
}
#endif

#endif // NITROARC_FFI_H
