/*
 * HustVec - C runtime implementation
 */

#include "hust_vec.h"
#include <stdlib.h>
#include <string.h>

/* Macro to implement vector functions for a specific type */
#define IMPLEMENT_HUST_VEC_TYPE(type, name) \
    HustVec_##name* HustVec_##name##_new(void) { \
        HustVec_##name* vec = (HustVec_##name*)malloc(sizeof(HustVec_##name)); \
        if (!vec) return NULL; \
        vec->data = NULL; \
        vec->len = 0; \
        vec->capacity = 0; \
        return vec; \
    } \
    \
    void HustVec_##name##_free(HustVec_##name* vec) { \
        if (!vec) return; \
        free(vec->data); \
        free(vec); \
    } \
    \
    void HustVec_##name##_push(HustVec_##name* vec, type value) { \
        if (!vec) return; \
        if (vec->len >= vec->capacity) { \
            size_t new_capacity = vec->capacity == 0 ? 4 : vec->capacity * 2; \
            type* new_data = (type*)realloc(vec->data, new_capacity * sizeof(type)); \
            if (!new_data) return; \
            vec->data = new_data; \
            vec->capacity = new_capacity; \
        } \
        vec->data[vec->len++] = value; \
    } \
    \
    type HustVec_##name##_get(const HustVec_##name* vec, size_t index) { \
        if (!vec || index >= vec->len) return (type){0}; \
        return vec->data[index]; \
    } \
    \
    void HustVec_##name##_set(HustVec_##name* vec, size_t index, type value) { \
        if (!vec || index >= vec->len) return; \
        vec->data[index] = value; \
    } \
    \
    size_t HustVec_##name##_len(const HustVec_##name* vec) { \
        if (!vec) return 0; \
        return vec->len; \
    } \
    \
    bool HustVec_##name##_is_empty(const HustVec_##name* vec) { \
        if (!vec) return true; \
        return vec->len == 0; \
    }

/* Implement for common types */
IMPLEMENT_HUST_VEC_TYPE(int32_t, i32)
IMPLEMENT_HUST_VEC_TYPE(int64_t, i64)
IMPLEMENT_HUST_VEC_TYPE(float, f32)
IMPLEMENT_HUST_VEC_TYPE(double, f64)
IMPLEMENT_HUST_VEC_TYPE(bool, bool)
