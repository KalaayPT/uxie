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
#include <stdlib.h> // only for qsort

#include "nitroarc_private.h"

static const char fntb_unnamed[] = {
    0x04, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x01, 0x00,
};

typedef struct sect sect_t;
struct sect {
    uint32_t offs;
    uint32_t head;
    uint32_t size;
};

#define NARC_HEADER (sect_t){ \
    .offs = 0,                \
    .head = NARC_HEADERSIZE,  \
    .size = 0,                \
}

#define sect(prev, head_size, data_size) (sect_t){ \
    .offs = prev.offs + prev.head + prev.size,     \
    .head = head_size,                             \
    .size = data_size,                             \
}

#define sectsize(sect) (sect.head + sect.size)

static uint32_t pack_fntb(nitroarc_packer_t *p, uint8_t **out_dirtab);

static int pack(nitroarc_packer_t *p, uint8_t *dirtab, uint32_t dirtab_size, uint8_t **out_buf, uint32_t *out_size);
static int pack_stripped(nitroarc_packer_t *p, uint8_t *dirtab, uint32_t dirtab_size, uint8_t **out_buf, uint32_t *out_size);

static void put_narc(void *base, uint32_t size);
static void put_fatb(void *base, uint32_t size, const nitroarc_packer_t *p);
static void put_fntb(void *base, uint32_t size, const void *_fntb);
static void put_fimg(void *base, uint32_t size, const void *_fimg);

int nitroarc_pseal(nitroarc_packer_t *p, void **out_data, uint32_t *out_size) {
    assert(p);
    assert(p->malloc);
    assert(p->realloc);
    assert(p->free);
    assert(out_data);
    assert(out_size);

    int      errc        = NITROARC_ESUCCESS;
    uint8_t *dirtab      = NULL;
    uint32_t dirtab_size = pack_fntb(p, &dirtab);
    if (dirtab == NULL) { errc = dirtab_size; goto cleanup; }

    uint32_t size = 0;
    uint8_t *data = NULL;
    errc = PACKCTL_ISSTRIPPED(p)
         ? pack_stripped(p, dirtab, dirtab_size, &data, &size)
         : pack(p, dirtab, dirtab_size, &data, &size);

    if (errc) goto cleanup;

    *out_data = data;
    *out_size = size;

cleanup:
    p->free(p->ctx, p->fimg_data, p->fimg_size, 1);
    p->free(p->ctx, p->f_records, p->f_count, sizeof(*p->f_records));
    if (PACKCTL_ISNAMED(p)) p->free(p->ctx, dirtab, dirtab_size, 1);
    return errc;
}

static int pack(nitroarc_packer_t *p, uint8_t *dirtab, uint32_t dirtab_size, uint8_t **out_buf, uint32_t *out_size) {
    sect_t narc = NARC_HEADER;
    sect_t fatb = sect(narc, FATB_HEADERSIZE, p->f_capacity * FATB_ENTRYSIZE);
    sect_t fntb = sect(fatb, FNTB_HEADERSIZE, dirtab_size);
    sect_t fimg = sect(fntb, FIMG_HEADERSIZE, p->fimg_size);

    size_t size = sectsize(narc)
                + sectsize(fatb)
                + sectsize(fntb)
                + sectsize(fimg);
    if (size >= UINT32_MAX) return NITROARC_ESIZEOVERFLOW;

    uint32_t usize = (uint32_t)size;
    uint8_t *data  = p->malloc(p->ctx, usize, 1);
    if (data == NULL) return NITROARC_EALLOCFAIL;

    put_narc(data + narc.offs, usize);
    put_fatb(data + fatb.offs, fatb.size, p);
    put_fntb(data + fntb.offs, fntb.size, dirtab);
    put_fimg(data + fimg.offs, fimg.size, p->fimg_data);

    *out_buf  = data;
    *out_size = usize;
    return NITROARC_ESUCCESS;
}

static int pack_stripped(nitroarc_packer_t *p, uint8_t *dirtab, uint32_t dirtab_size, uint8_t **out_buf, uint32_t *out_size) {
    uint32_t head_size  = 0x10;
    uint32_t fntb_align = align_u32(head_size + dirtab_size);
    uint32_t fntb_offs  = head_size;
    uint32_t fntb_size  = dirtab_size + fntb_align;
    uint32_t fatb_offs  = head_size + fntb_size;
    uint32_t fatb_size  = p->f_capacity * FATB_ENTRYSIZE;
    uint32_t fatb_align = align_u32(fatb_offs + fatb_size);
    uint32_t fimg_offs  = fatb_offs + fatb_size + fatb_align;

    size_t size = fimg_offs + p->fimg_size;
    if (size >= UINT32_MAX) return NITROARC_ESIZEOVERFLOW;

    uint32_t usize = (uint32_t)size;
    uint8_t *data  = p->malloc(p->ctx, usize, 1);
    if (data == NULL) return NITROARC_EALLOCFAIL;

    putleu32(&data[0x00], fntb_offs);
    putleu32(&data[0x04], dirtab_size);
    putleu32(&data[0x08], fatb_offs);
    putleu32(&data[0x0C], fatb_size);

    uint8_t *fntb = &data[fntb_offs];
    uint8_t *fatb = &data[fatb_offs];
    uint8_t *fimg = &data[fimg_offs];

    for (size_t i = 0; i < dirtab_size; i++)         fntb[i] = dirtab[i];
    for (size_t i = dirtab_size; i < fntb_size; i++) fntb[i] = 0;

    for (size_t i = fatb_size; i < fatb_align; i++) fatb[i] = 0;
    for (size_t i = 0; i < p->f_count; i++) {
        nitroarc_file_t *p_file  = &p->f_records[i];
        unsigned char   *p_entry = &fatb[i * FATB_ENTRYSIZE];

        putleu32(&p_entry[0x00], p_file->offset + fimg_offs);
        putleu32(&p_entry[0x04], p_file->offset + fimg_offs + p_file->size);
    }

    for (size_t i = 0; i < p->fimg_size; i++) {
        fimg[i] = ((uint8_t *)p->fimg_data)[i];
    }

    *out_buf  = data;
    *out_size = usize;
    return NITROARC_ESUCCESS;
}

static void put_narc(void *base, uint32_t size) {
    unsigned char *data = (unsigned char *)base;

    putleu32(&data[0x00], SIGNATURE_NARC);
    putleu16(&data[0x04], BOM_LITEND);
    putleu16(&data[0x06], NARC_VERSION);
    putleu32(&data[0x08], size);
    putleu16(&data[0x0C], NARC_HEADERSIZE);
    putleu16(&data[0x0E], NARC_SECTIONS);
}

static void put_fatb(void *base, uint32_t size, const nitroarc_packer_t *p) {
    unsigned char *head = (unsigned char *)base;
    unsigned char *data = head + FATB_HEADERSIZE;

    putleu32(&head[0x00], SIGNATURE_BTAF);
    putleu32(&head[0x04], size + FATB_HEADERSIZE);
    putleu16(&head[0x08], p->f_count);
    putleu16(&head[0x0A], 0);

    for (size_t i = 0; i < p->f_count; i++) {
        nitroarc_file_t *p_file  = &p->f_records[i];
        unsigned char   *p_entry = &data[i * FATB_ENTRYSIZE];

        putleu32(&p_entry[0x00], p_file->offset);
        putleu32(&p_entry[0x04], p_file->offset + p_file->size);
    }
}

static void put_fntb(void *base, uint32_t size, const void *_fntb) {
    unsigned char *head = (unsigned char *)base;
    unsigned char *data = head + FNTB_HEADERSIZE;
    unsigned char *fntb = (unsigned char *)_fntb;

    putleu32(&head[0x00], SIGNATURE_BTNF);
    putleu32(&head[0x04], size + FNTB_HEADERSIZE);

    for (size_t i = 0; i < size; i++) data[i] = fntb[i];
}

static void put_fimg(void *base, uint32_t size, const void *_fimg) {
    unsigned char *head = (unsigned char *)base;
    unsigned char *data = head + FIMG_HEADERSIZE;
    unsigned char *fimg = (unsigned char *)_fimg;

    putleu32(&head[0x00], SIGNATURE_GMIF);
    putleu32(&head[0x04], size + FIMG_HEADERSIZE);

    for (size_t i = 0; i < size; i++) data[i] = fimg[i];
}

// ============================ FNTB PACKING CODE =========================== //

typedef struct fsnode fsnode_t;
struct fsnode {
    char     *name;
    fsnode_t *children;

    uint16_t id;
    uint16_t parent;
    uint16_t n_children;
    uint16_t c_children;
    uint16_t first;
    uint8_t  name_len;
};

static int      filecmp(const void *lhs, const void *rhs);
static void     flatten(fsnode_t *parent, fsnode_t **out);
static unsigned calcenttab(fsnode_t **dirs, uint16_t ndirs, uint16_t nfiles);
static void     serialize(fsnode_t **dirs, char *dirtab, char *enttab);

static int  fs_push(nitroarc_packer_t *p, fsnode_t *root, char *path, uint16_t *did, uint16_t *fid);
static void fs_free(nitroarc_packer_t *p, fsnode_t *parent);

#define INIT_CAP 32

static uint32_t pack_fntb(nitroarc_packer_t *p, uint8_t **out_fntb) {
    if (!PACKCTL_ISNAMED(p)) {
        *out_fntb = (uint8_t *)fntb_unnamed;
        return sizeof(fntb_unnamed);
    }

    *out_fntb        = NULL;
    uint16_t dir_id  = 0xF000;
    uint16_t n_files = 0;
    fsnode_t root    = {
        .name        = "",
        .children    = p->malloc(p->ctx, INIT_CAP, sizeof(fsnode_t)),
        .id          = dir_id++,
        .parent      = 0,
        .first       = n_files,
        .n_children  = 0,
        .c_children  = INIT_CAP,
        .name_len    = 0,
    };

    qsort(p->f_records, p->f_count, sizeof(nitroarc_file_t), filecmp);

    fsnode_t **dirs  = NULL;
    char      *fntb  = NULL;
    unsigned   size  = 0;
    int        errc  = NITROARC_ESUCCESS;

    for (size_t i = 0; i < p->f_count; i++) {
        errc = fs_push(p, &root, p->f_records[i].name, &dir_id, &n_files);
        if (errc) goto erralloc;
    }

    root.parent = (dir_id & FNTB_FILESYS_MASKID);
    dirs        = p->malloc(p->ctx, root.parent, sizeof(fsnode_t*));

    if (dirs == NULL) { errc = NITROARC_EALLOCFAIL; goto erralloc; }
    flatten(&root, dirs);

    unsigned size_dirtab = root.parent * FNTB_DIRTAB_ENTRYSIZE;
    unsigned size_enttab = calcenttab(dirs, root.parent, n_files);

    size         = size_dirtab + size_enttab;
    fntb         = p->malloc(p->ctx, size, 1);
    char *dirtab = fntb;
    char *enttab = fntb + size_dirtab;

    serialize(dirs, dirtab, enttab);

    *out_fntb = (uint8_t *)fntb;
    errc      = size;
    goto cleanup;

erralloc:
    p->free(p->ctx, fntb, size, 1);

cleanup:
    p->free(p->ctx, dirs, root.parent, sizeof(fsnode_t*));
    fs_free(p, &root);
    return errc;
}

static char lower(char c) {
    return (c >= 'A' && c <= 'Z') ? c + ('a' - 'A') : c;
}

static int filecmp(const void *_lhs, const void *_rhs) {
    const nitroarc_file_t *file_l = _lhs;
    const nitroarc_file_t *file_r = _rhs;

    const char *lhs = file_l->name;
    const char *rhs = file_r->name;

    int result = 0;
    for (; result == 0 && *lhs && *rhs; lhs++, rhs++) {
        const char *sep_l = strchrnul(lhs, '/');
        const char *sep_r = strchrnul(rhs, '/');

        // always sort directories after files
        if (*sep_l == '/' && *sep_r != '/') return  1;
        if (*sep_l != '/' && *sep_r == '/') return -1;

        for (; result == 0 && lhs != sep_l && rhs != sep_r; lhs++, rhs++) {
            result = lower(*lhs) - lower(*rhs);
        }

        if (result)       return result;
        if (lhs != sep_l) return  1; // rhs is shorter
        if (rhs != sep_r) return -1; // lhs is shorter
    }

    return result;
}

static void flatten(fsnode_t *parent, fsnode_t **out) {
    uint16_t id = parent->id & 0x0FFF;
    out[id]     = parent;

    for (size_t i = 0; i < parent->n_children; i++) {
        fsnode_t *child = &parent->children[i];
        if (child->id >= 0xF000) flatten(child, out);
    }
}

static unsigned calcenttab(fsnode_t **dirs, uint16_t ndirs, uint16_t nfiles) {
    unsigned total = 0;
    for (int i = 0; i < ndirs; i++) {
        total += dirs[i]->name_len;

        for (int j = 0; j < dirs[i]->n_children; j++) {
            if (dirs[i]->children[j].children == NULL) {
                total += dirs[i]->children[j].name_len;
            }
        }
    }

    return total            // Total length of all unique components
        + ((ndirs - 1) * 2) // 2 bytes for each subdirectory ID
        + (ndirs - 1)       // 1 byte for each subdirectory's name-length
        + nfiles            // 1 byte for each file's name-length
        + ndirs;            // 1 byte to zero-terminate each entry
}

static void record(fsnode_t *dir, char *p_dirtab, uint32_t offset) {
    putleu32(&p_dirtab[0], offset);
    putleu16(&p_dirtab[4], dir->first);
    putleu16(&p_dirtab[6], dir->parent);
}

static void serialize(fsnode_t **dirs, char *dirtab, char *enttab) {
    assert(dirs);
    assert(dirs[0]);

    uint16_t n_dirs   = dirs[0]->parent;
    char    *p_dirtab = dirtab;
    char    *p_enttab = enttab;

    for (int i = 0; i < n_dirs; i++) {
        fsnode_t *dir = dirs[i];
        record(dir, p_dirtab, (uint32_t)(p_enttab - dirtab));

        for (int j = 0; j < dir->n_children; j++) {
            fsnode_t *child = &dir->children[j];
            *p_enttab++     = child->name_len | ((child->id >= 0xF000) * FNTB_DIRENT_MASKTYPE);

            for (int k = 0; k < child->name_len; k++) {
                *p_enttab++ = child->name[k];
            }

            if (child->id >= 0xF000) {
                putleu16(p_enttab, child->id);
                p_enttab += 2;
            }
        }

        *p_enttab = 0;
        p_dirtab += FNTB_DIRTAB_ENTRYSIZE;
        p_enttab += 1;
    }
}

static void fs_free(nitroarc_packer_t *p, fsnode_t *parent) {
    for (size_t i = 0; i < parent->n_children; i++) {
        fs_free(p, &parent->children[i]);
    }

    p->free(p->ctx, parent->children, parent->n_children, sizeof(fsnode_t));
}

static int fs_pushone(
    nitroarc_packer_t *p,
    fsnode_t  *parent,
    char      *path,
    uint8_t    path_len,
    uint16_t  *id,
    uint16_t   first,
    fsnode_t **out_child
) {
    assert(parent);
    assert(path);
    assert(id);
    assert(out_child);

    *out_child = NULL;

    for (size_t i = 0; i < parent->n_children; i++) {
        fsnode_t *child = &parent->children[i];

        char *p = path;
        char *s = child->name;
        for (; *p && *s && *p == *s; p++, s++);

        if (*p == *s) {
            *out_child = child;
            return NITROARC_ESUCCESS;
        }
    }

    // no match found; add this as a child
    if (parent->n_children + 1 >= parent->c_children) {
        int cap = parent->c_children * 2;
        if (cap > UINT16_MAX) return NITROARC_EALLOCFAIL;

        fsnode_t *tmp = p->realloc(p->ctx, parent->children, cap, sizeof(fsnode_t));
        if (tmp == NULL) return NITROARC_EALLOCFAIL;

        parent->c_children = (uint16_t)cap;
        parent->children   = tmp;
    }

    fsnode_t *child   = &parent->children[parent->n_children++];
    child->name       = path;
    child->children   = NULL;
    child->id         = *id;
    child->parent     = parent->id;
    child->first      = first;
    child->n_children = 0;
    child->c_children = 0;
    child->name_len   = path_len;

    if (*id >= 0xF000) {
        child->c_children = INIT_CAP;
        child->children   = p->malloc(p->ctx, INIT_CAP, sizeof(fsnode_t));
        if (child->children == NULL) return NITROARC_EALLOCFAIL;
    }

    (*id)++;
    *out_child = child;
    return NITROARC_ESUCCESS;
}

static int fs_push(
    nitroarc_packer_t *p,
    fsnode_t *root,
    char     *path,
    uint16_t *did,
    uint16_t *fid
) {
    int       errc  = NITROARC_ESUCCESS;
    fsnode_t *child = NULL;

    char *s;
    for (s = path; s && *s; s++) {
        if (*s == '/') {
            *s   = 0;
            errc = fs_pushone(p, root, path, (uint8_t)(s - path), did, *fid, &child);
            if (errc) return errc;

            path   = s + 1;
            root   = child;
        }
    }

    errc = fs_pushone(p, root, path, (uint8_t)(s - path), fid, 0, &child);
    if (errc) return errc;

    return NITROARC_ESUCCESS;
}

#endif
