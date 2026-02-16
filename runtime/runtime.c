#include <stdio.h>
#include <stdint.h>
#include <stddef.h>

// TRUE=-1 / FALSE=0 の表示規則
static const char* bool_str(int32_t x) { return (x == -1) ? "TRUE" : "FALSE"; }

#define MEM_CELLS 65536
static int32_t mem_cells[MEM_CELLS];

static int32_t clamp_idx(int32_t idx) {
  if (idx < 0) return 0;
  if (idx >= MEM_CELLS) return MEM_CELLS - 1;
  return idx;
}

void pwrite_i32(int32_t x) { printf("%d", x); }
void pwrite_bool(int32_t x) { printf("%s", bool_str(x)); }
void pwrite_char(int32_t x) { putchar((unsigned char)(x & 0xFF)); }
void pwrite_hex(int32_t x) { printf("%08x", (uint32_t)x); }
void pwriteln(void) { putchar('\n'); }
void pwrite_str(const char* s) { fputs(s, stdout); }

int32_t pread_i32(void) { int32_t x; if (scanf("%d", &x) != 1) return 0; return x; }
int32_t pread_bool(void) { int32_t x; if (scanf("%d", &x) != 1) return 0; return x ? -1 : 0; }
int32_t pread_char(void) { int c = getchar(); if (c == EOF) return 0; return (int32_t)(unsigned char)c; }
void preadln(void) { int c; while ((c=getchar()) != '\n' && c != EOF) {} }

// ひとまず未実装（必要になったら設計して追加）
int32_t pvar_get(int32_t id) {
  int32_t idx = clamp_idx(id / 4);
  return mem_cells[idx];
}
void pvar_set(int32_t v, int32_t id) {
  int32_t idx = clamp_idx(id / 4);
  mem_cells[idx] = v;
}
int32_t pfield_get(int32_t obj, int32_t off) {
  int32_t idx = clamp_idx((obj + off) / 4);
  return mem_cells[idx];
}
void pfield_set(int32_t v, int32_t obj, int32_t off) {
  int32_t idx = clamp_idx((obj + off) / 4);
  mem_cells[idx] = v;
}

int32_t pbool(int32_t x) { return x ? -1 : 0; }
