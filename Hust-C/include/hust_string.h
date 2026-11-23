/*
 * hust_string.h - String runtime for Hust-C
 *
 * String is a language-level type (r3), not a member of C.string.
 */

#ifndef HUST_STRING_H
#define HUST_STRING_H

#include <stdbool.h>
#include <stddef.h>
#include <stdlib.h>
#include <string.h>

typedef struct HustString {
    char* data;
    size_t len;
} HustString;

static inline HustString* HustString_new(const char* src) {
    HustString* s = (HustString*)malloc(sizeof(HustString));
    if (!s) return NULL;
    s->len = src ? strlen(src) : 0;
    s->data = (char*)malloc(s->len + 1);
    if (!s->data) {
        free(s);
        return NULL;
    }
    if (src) {
        memcpy(s->data, src, s->len + 1);
    } else {
        s->data[0] = '\0';
    }
    return s;
}

static inline void HustString_free(HustString* s) {
    if (!s) return;
    free(s->data);
    free(s);
}

static inline size_t HustString_len(const HustString* s) {
    return s ? s->len : 0;
}

static inline size_t HustString_length(const HustString* s) {
    return HustString_len(s);
}

static inline const char* HustString_cstr(const HustString* s) {
    return s ? s->data : "";
}

static inline bool HustString_equals(const HustString* a, const HustString* b) {
    if (a == b) return true;
    if (!a || !b) return false;
    if (a->len != b->len) return false;
    return memcmp(a->data, b->data, a->len) == 0;
}

static inline bool HustString_equals_cstr(const HustString* a, const char* b) {
    if (!a || !b) return false;
    return strcmp(a->data, b) == 0;
}

static inline HustString* HustString_concat(const HustString* a, const HustString* b) {
    size_t alen = a ? a->len : 0;
    size_t blen = b ? b->len : 0;
    HustString* out = (HustString*)malloc(sizeof(HustString));
    if (!out) return NULL;
    out->data = (char*)malloc(alen + blen + 1);
    if (!out->data) {
        free(out);
        return NULL;
    }
    if (alen > 0) memcpy(out->data, a->data, alen);
    if (blen > 0) memcpy(out->data + alen, b->data, blen);
    out->data[alen + blen] = '\0';
    out->len = alen + blen;
    return out;
}

#endif /* HUST_STRING_H */
