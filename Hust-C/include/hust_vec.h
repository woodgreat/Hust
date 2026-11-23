/*
 * hust_vec.h - Dynamic array runtime for Hust-C
 *
 * Language-level runtime: String and dynamic arrays are core features,
 * not members of C.* modules.
 */

#ifndef HUST_VEC_H
#define HUST_VEC_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

#define HUST_VEC_DECLARE(TYPE, NAME) \
typedef struct { \
    TYPE* data; \
    size_t size; \
    size_t capacity; \
} HustVec_##NAME; \
\
static inline HustVec_##NAME HustVec_##NAME##_new(void) { \
    HustVec_##NAME vec; \
    vec.data = NULL; \
    vec.size = 0; \
    vec.capacity = 0; \
    return vec; \
} \
\
static inline void HustVec_##NAME##_free(HustVec_##NAME* vec) { \
    if (!vec) return; \
    free(vec->data); \
    vec->data = NULL; \
    vec->size = 0; \
    vec->capacity = 0; \
} \
\
static inline bool HustVec_##NAME##_push(HustVec_##NAME* vec, TYPE value) { \
    if (!vec) return false; \
    if (vec->size >= vec->capacity) { \
        size_t new_capacity = (vec->capacity == 0) ? 4 : vec->capacity * 2; \
        TYPE* new_data = (TYPE*)realloc(vec->data, new_capacity * sizeof(TYPE)); \
        if (!new_data) return false; \
        vec->data = new_data; \
        vec->capacity = new_capacity; \
    } \
    vec->data[vec->size++] = value; \
    return true; \
} \
\
static inline bool HustVec_##NAME##_pop(HustVec_##NAME* vec) { \
    if (!vec || vec->size == 0) return false; \
    vec->size--; \
    return true; \
} \
\
static inline size_t HustVec_##NAME##_len(const HustVec_##NAME* vec) { \
    return vec ? vec->size : 0; \
} \
\
static inline bool HustVec_##NAME##_get(const HustVec_##NAME* vec, size_t index, TYPE* out_value) { \
    if (!vec || index >= vec->size) return false; \
    if (out_value) *out_value = vec->data[index]; \
    return true; \
} \
\
static inline bool HustVec_##NAME##_set(HustVec_##NAME* vec, size_t index, TYPE value) { \
    if (!vec || index >= vec->size) return false; \
    vec->data[index] = value; \
    return true; \
}

HUST_VEC_DECLARE(int8_t, i8)
HUST_VEC_DECLARE(int16_t, i16)
HUST_VEC_DECLARE(int32_t, i32)
HUST_VEC_DECLARE(int64_t, i64)
HUST_VEC_DECLARE(uint8_t, u8)
HUST_VEC_DECLARE(uint16_t, u16)
HUST_VEC_DECLARE(uint32_t, u32)
HUST_VEC_DECLARE(uint64_t, u64)
HUST_VEC_DECLARE(float, f32)
HUST_VEC_DECLARE(double, f64)
HUST_VEC_DECLARE(bool, bool)
HUST_VEC_DECLARE(char, char)

/* String vectors use the language-level HustString runtime. */
struct HustString;
HUST_VEC_DECLARE(struct HustString*, String)

/* Nested vectors for dynamic 2D/3D arrays. */
HUST_VEC_DECLARE(HustVec_i32, HustVec_i32)
HUST_VEC_DECLARE(HustVec_HustVec_i32, HustVec_HustVec_i32)
HUST_VEC_DECLARE(HustVec_HustVec_HustVec_i32, HustVec_HustVec_HustVec_i32)
#endif /* HUST_VEC_H */
