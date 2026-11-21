#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
    int32_t width;
    int32_t height;
} Rectangle;

Rectangle* Rectangle_new(int32_t w, int32_t h);
void Rectangle_free(Rectangle* self);
int32_t Rectangle_getArea(Rectangle* self);
void Rectangle_scale(Rectangle* self, int32_t factor);

Rectangle* Rectangle_new(int32_t w, int32_t h) {
    Rectangle* self = (Rectangle*)malloc(sizeof(Rectangle));
    if (self == NULL) return NULL;
    self->width = w;
    self->height = h;
    return self;
}

void Rectangle_free(Rectangle* self) {
    free(self);
}

int32_t Rectangle_getArea(Rectangle* self) {
    return self->width * self->height;
}

void Rectangle_scale(Rectangle* self, int32_t factor) {
    self->width = self->width * factor;
    self->height = self->height * factor;
}

void main(void) {
    Rectangle* rect = Rectangle_new(10, 20);
    int32_t area = Rectangle_getArea(rect);
    Rectangle_scale(rect, 2);
    Rectangle_free(rect);
}
