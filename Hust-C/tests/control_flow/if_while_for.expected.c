#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

void main(void) {
    int32_t x = 10;

    if (x > 5) {
        printf("x is greater than 5\n");
    } else if (x == 5) {
        printf("x is equal to 5\n");
    } else {
        printf("x is less than 5\n");
    }

    while (x > 0) {
        printf("x = %d\n", x);
        x = x - 1;
    }

    for (int32_t i = 0; i < 5; i++) {
        printf("i = %d\n", i);
    }
}
