#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int32_t add(int32_t a, int32_t b) {
    return a + b;
}

void greet(HustString* name) {
    printf("Hello, %s!\n", HustString_cstr(name));
}

void main(void) {
    int32_t result = add(1, 2);
    greet("World");
}
