#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

void main(void) {
    // Fixed array
    int32_t scores[5] = {85, 90, 78, 92, 88};
    int32_t first = scores[0];
    scores[1] = 95;

    // Dynamic array
    HustVec_i32* numbers = HustVec_i32_new();
    HustVec_i32_push(numbers, 10);
    HustVec_i32_push(numbers, 20);
    HustVec_i32_push(numbers, 30);
    size_t len = HustVec_i32_len(numbers);
    int32_t val = HustVec_i32_get(numbers, 1);

    // Multi-dimensional array
    int32_t matrix[3][4];
    matrix[0][0] = 1;
    matrix[2][3] = 12;
}
