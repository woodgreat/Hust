/*
 * hust_vec.h - Dynamic array (vector) implementation for Hust-C
 * 
 * Provides HustVec structure and operations for dynamic arrays
 */

#ifndef HUST_VEC_H
#define HUST_VEC_H

#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

/* Dynamic array structure for int32_t */
typedef struct {
    int32_t* data;
    size_t size;
    size_t capacity;
} HustVec_i32;

/* Dynamic array structure for int64_t */
typedef struct {
    int64_t* data;
    size_t size;
    size_t capacity;
} HustVec_i64;

/* Dynamic array structure for float */
typedef struct {
    float* data;
    size_t size;
    size_t capacity;
} HustVec_f32;

/* Dynamic array structure for double */
typedef struct {
    double* data;
    size_t size;
    size_t capacity;
} HustVec_f64;

/* Dynamic array structure for bool */
typedef struct {
    bool* data;
    size_t size;
    size_t capacity;
} HustVec_bool;

/* ============ HustVec_i32 Operations ============ */

/* Create a new HustVec_i32 */
static inline HustVec_i32* HustVec_i32_new(void) {
    HustVec_i32* vec = (HustVec_i32*)malloc(sizeof(HustVec_i32));
    if (!vec) return NULL;
    vec->data = NULL;
    vec->size = 0;
    vec->capacity = 0;
    return vec;
}

/* Free HustVec_i32 */
static inline void HustVec_i32_free(HustVec_i32* vec) {
    if (!vec) return;
    free(vec->data);
    free(vec);
}

/* Push element to HustVec_i32 */
static inline bool HustVec_i32_push(HustVec_i32* vec, int32_t value) {
    if (!vec) return false;
    
    /* Grow if needed */
    if (vec->size >= vec->capacity) {
        size_t new_capacity = (vec->capacity == 0) ? 4 : vec->capacity * 2;
        int32_t* new_data = (int32_t*)realloc(vec->data, new_capacity * sizeof(int32_t));
        if (!new_data) return false;
        vec->data = new_data;
        vec->capacity = new_capacity;
    }
    
    vec->data[vec->size++] = value;
    return true;
}

/* Pop element from HustVec_i32 (returns popped value, sets success flag) */
static inline bool HustVec_i32_pop(HustVec_i32* vec, int32_t* out_value) {
    if (!vec || vec->size == 0) return false;
    if (out_value) *out_value = vec->data[vec->size - 1];
    vec->size--;
    return true;
}

/* Get length of HustVec_i32 */
static inline size_t HustVec_i32_len(const HustVec_i32* vec) {
    return vec ? vec->size : 0;
}

/* Get element at index (with bounds check) */
static inline bool HustVec_i32_get(const HustVec_i32* vec, size_t index, int32_t* out_value) {
    if (!vec || index >= vec->size) return false;
    if (out_value) *out_value = vec->data[index];
    return true;
}

/* Set element at index (with bounds check) */
static inline bool HustVec_i32_set(HustVec_i32* vec, size_t index, int32_t value) {
    if (!vec || index >= vec->size) return false;
    vec->data[index] = value;
    return true;
}

/* ============ HustVec_f64 Operations ============ */

/* Create a new HustVec_f64 */
static inline HustVec_f64* HustVec_f64_new(void) {
    HustVec_f64* vec = (HustVec_f64*)malloc(sizeof(HustVec_f64));
    if (!vec) return NULL;
    vec->data = NULL;
    vec->size = 0;
    vec->capacity = 0;
    return vec;
}

/* Free HustVec_f64 */
static inline void HustVec_f64_free(HustVec_f64* vec) {
    if (!vec) return;
    free(vec->data);
    free(vec);
}

/* Push element to HustVec_f64 */
static inline bool HustVec_f64_push(HustVec_f64* vec, double value) {
    if (!vec) return false;
    
    if (vec->size >= vec->capacity) {
        size_t new_capacity = (vec->capacity == 0) ? 4 : vec->capacity * 2;
        double* new_data = (double*)realloc(vec->data, new_capacity * sizeof(double));
        if (!new_data) return false;
        vec->data = new_data;
        vec->capacity = new_capacity;
    }
    
    vec->data[vec->size++] = value;
    return true;
}

/* Pop element from HustVec_f64 */
static inline bool HustVec_f64_pop(HustVec_f64* vec, double* out_value) {
    if (!vec || vec->size == 0) return false;
    if (out_value) *out_value = vec->data[vec->size - 1];
    vec->size--;
    return true;
}

/* Get length of HustVec_f64 */
static inline size_t HustVec_f64_len(const HustVec_f64* vec) {
    return vec ? vec->size : 0;
}

/* Get element at index */
static inline bool HustVec_f64_get(const HustVec_f64* vec, size_t index, double* out_value) {
    if (!vec || index >= vec->size) return false;
    if (out_value) *out_value = vec->data[index];
    return true;
}

/* Set element at index */
static inline bool HustVec_f64_set(HustVec_f64* vec, size_t index, double value) {
    if (!vec || index >= vec->size) return false;
    vec->data[index] = value;
    return true;
}

/* ============ Generic Macros for Type-Specific Vectors ============ */

/* Declare a new vector type for a specific element type */
#define HUST_VEC_DECLARE(TYPE, NAME) \
typedef struct { \
    TYPE* data; \
    size_t size; \
    size_t capacity; \
} HustVec_##NAME; \
\
static inline HustVec_##NAME* HustVec_##NAME##_new(void) { \
    HustVec_##NAME* vec = (HustVec_##NAME*)malloc(sizeof(HustVec_##NAME)); \
    if (!vec) return NULL; \
    vec->data = NULL; \
    vec->size = 0; \
    vec->capacity = 0; \
    return vec; \
} \
\
static inline void HustVec_##NAME##_free(HustVec_##NAME* vec) { \
    if (!vec) return; \
    free(vec->data); \
    free(vec); \
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
static inline bool HustVec_##NAME##_pop(HustVec_##NAME* vec, TYPE* out_value) { \
    if (!vec || vec->size == 0) return false; \
    if (out_value) *out_value = vec->data[vec->size - 1]; \
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

/* Declare common vector types */
HUST_VEC_DECLARE(int64_t, i64)
HUST_VEC_DECLARE(float, f32)
HUST_VEC_DECLARE(bool, bool)
HUST_VEC_DECLARE(char, char)

#endif /* HUST_VEC_H */
