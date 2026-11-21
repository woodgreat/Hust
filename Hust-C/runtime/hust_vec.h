/*
 * HustVec - C runtime for Hust dynamic array type
 * 
 * This provides a dynamic array implementation for Hust code
 * transpiled to C.
 */

#ifndef HUST_VEC_H
#define HUST_VEC_H

#include <stddef.h>
#include <stdbool.h>
#include <stdint.h>

/* Macro to define vector types for different element types */
#define DEFINE_HUST_VEC_TYPE(type, name) \
    typedef struct { \
        type* data; \
        size_t len; \
        size_t capacity; \
    } HustVec_##name; \
    \
    HustVec_##name* HustVec_##name##_new(void); \
    void HustVec_##name##_free(HustVec_##name* vec); \
    void HustVec_##name##_push(HustVec_##name* vec, type value); \
    type HustVec_##name##_get(const HustVec_##name* vec, size_t index); \
    void HustVec_##name##_set(HustVec_##name* vec, size_t index, type value); \
    size_t HustVec_##name##_len(const HustVec_##name* vec); \
    bool HustVec_##name##_is_empty(const HustVec_##name* vec);

/* Define common vector types */
DEFINE_HUST_VEC_TYPE(int32_t, i32)
DEFINE_HUST_VEC_TYPE(int64_t, i64)
DEFINE_HUST_VEC_TYPE(float, f32)
DEFINE_HUST_VEC_TYPE(double, f64)
DEFINE_HUST_VEC_TYPE(bool, bool)

#endif /* HUST_VEC_H */
