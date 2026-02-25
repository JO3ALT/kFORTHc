#include <ctype.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <math.h>
#include <stdlib.h>

// kforth compatibility: any non-zero is true for display/branching helper words.
static const char* bool_str(int32_t x) { return x ? "TRUE" : "FALSE"; }

#define MEM_CELLS 65536
static int32_t mem_cells[MEM_CELLS];
static int32_t g_here_bytes = 0;

static int g_pushback = -1;

static const float KP_PI_F = 3.14159265358979323846f;
static const float KP_DEG2RAD_F = 3.14159265358979323846f / 180.0f;
static const float KP_RAD2DEG_F = 180.0f / 3.14159265358979323846f;
static const float KP_FIX_SCALE_F = 10000.0f;

static int rt_getc1(void) {
  if (g_pushback >= 0) {
    int c = g_pushback;
    g_pushback = -1;
    return c;
  }
  return getchar();
}

static void rt_ungetc1(int c) {
  g_pushback = c;
}

static int next_token(char* buf, size_t cap) {
  size_t n = 0;
  int c;

  do {
    c = rt_getc1();
    if (c == EOF) return 0;
  } while (isspace((unsigned char)c));

  while (c != EOF && !isspace((unsigned char)c)) {
    if (n + 1 < cap) buf[n++] = (char)c;
    c = rt_getc1();
  }
  if (c != EOF) rt_ungetc1(c);
  buf[n] = '\0';
  return 1;
}

static int32_t clamp_idx(int32_t idx) {
  if (idx < 0) return 0;
  if (idx >= MEM_CELLS) return MEM_CELLS - 1;
  return idx;
}

void rt_heap_reset(int32_t base) {
  if (base < 0) base = 0;
  if (base > MEM_CELLS * 4) base = MEM_CELLS * 4;
  g_here_bytes = base;
}
int32_t rt_here(void) { return g_here_bytes; }
void rt_allot(int32_t n) {
  int64_t next = (int64_t)g_here_bytes + (int64_t)n;
  if (next < 0) next = 0;
  if (next > (int64_t)MEM_CELLS * 4) next = (int64_t)MEM_CELLS * 4;
  g_here_bytes = (int32_t)next;
}

static float bits_to_f32(int32_t bits) {
  union {
    uint32_t u;
    float f;
  } v;
  v.u = (uint32_t)bits;
  return v.f;
}

static int32_t f32_to_bits(float f) {
  union {
    uint32_t u;
    float f;
  } v;
  v.f = f;
  return (int32_t)v.u;
}

static int32_t forth_bool(int cond) { return cond ? -1 : 0; }

static uint32_t fexp_raw_u32(uint32_t u) { return (u >> 23) & 0xFFu; }
static uint32_t ffrac_u32(uint32_t u) { return u & 0x7FFFFFu; }
static int is_nan_bits_u32(uint32_t u) { return fexp_raw_u32(u) == 0xFFu && ffrac_u32(u) != 0; }
static int is_inf_bits_u32(uint32_t u) { return fexp_raw_u32(u) == 0xFFu && ffrac_u32(u) == 0; }
static int is_finite_bits_u32(uint32_t u) { return fexp_raw_u32(u) != 0xFFu; }

void pwrite_i32(int32_t x) { printf("%d", x); }
void pwrite_bool(int32_t x) { printf("%s", bool_str(x)); }
void pwrite_char(int32_t x) { putchar((unsigned char)(x & 0xFF)); }
void pwrite_hex(int32_t x) { printf("%08X", (uint32_t)x); }
void pwriteln(void) { putchar('\n'); }
void pwrite_str(const char* s) { fputs(s, stdout); }
int32_t pbool(int32_t x);

int32_t pread_i32(void) {
  char tok[256];
  char* end = NULL;
  long v;
  if (!next_token(tok, sizeof(tok))) return 0;
  v = strtol(tok, &end, 10);
  if (end == tok || *end != '\0') return 0;
  return (int32_t)v;
}

int32_t pread_bool(void) {
  return pbool(pread_i32());
}

int32_t pread_char(void) {
  char tok[256];
  char* end = NULL;
  long v;
  if (!next_token(tok, sizeof(tok))) return 0;
  if (tok[0] != '\0' && tok[1] == '\0') return (unsigned char)tok[0];
  v = strtol(tok, &end, 10);
  if (end == tok || *end != '\0') return 0;
  return (int32_t)v;
}

int32_t pread_f32_bits(void) {
  char tok[256];
  char* end = NULL;
  float v;
  if (!next_token(tok, sizeof(tok))) return 0;
  v = strtof(tok, &end);
  if (end == tok || *end != '\0') return 0;
  return f32_to_bits(v);
}

void preadln(void) {
  int c;
  while ((c = rt_getc1()) != '\n' && c != EOF) {
  }
}

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

int32_t pbool(int32_t x) { return x ? 1 : 0; }

void pwrite_f32_bits(int32_t bits) {
  uint32_t u = (uint32_t)bits;
  if (is_nan_bits_u32(u)) {
    fputs("nan", stdout);
    return;
  }
  if (is_inf_bits_u32(u)) {
    if (u >> 31) {
      fputs("-inf", stdout);
    } else {
      fputs("inf", stdout);
    }
    return;
  }
  printf("%.4f", bits_to_f32(bits));
}

int32_t fadd_bits(int32_t a, int32_t b) { return f32_to_bits(bits_to_f32(a) + bits_to_f32(b)); }
int32_t fsub_bits(int32_t a, int32_t b) { return f32_to_bits(bits_to_f32(a) - bits_to_f32(b)); }
int32_t fmul_bits(int32_t a, int32_t b) { return f32_to_bits(bits_to_f32(a) * bits_to_f32(b)); }
int32_t fdiv_bits(int32_t a, int32_t b) { return f32_to_bits(bits_to_f32(a) / bits_to_f32(b)); }

int32_t fnegate_bits(int32_t a) { return (int32_t)(((uint32_t)a) ^ 0x80000000u); }
int32_t fabs_bits(int32_t a) { return (int32_t)(((uint32_t)a) & 0x7FFFFFFFu); }

int32_t feq_bits(int32_t a, int32_t b) {
  float fa = bits_to_f32(a), fb = bits_to_f32(b);
  return forth_bool(fa == fb);
}
int32_t flt_bits(int32_t a, int32_t b) {
  float fa = bits_to_f32(a), fb = bits_to_f32(b);
  return forth_bool(fa < fb);
}
int32_t fle_bits(int32_t a, int32_t b) {
  float fa = bits_to_f32(a), fb = bits_to_f32(b);
  return forth_bool(fa <= fb);
}

int32_t fzero_bits(int32_t a) { return forth_bool((((uint32_t)a) & 0x7FFFFFFFu) == 0); }
int32_t finf_bits(int32_t a) { return forth_bool(is_inf_bits_u32((uint32_t)a)); }
int32_t fnan_bits(int32_t a) { return forth_bool(is_nan_bits_u32((uint32_t)a)); }
int32_t ffinite_bits(int32_t a) { return forth_bool(is_finite_bits_u32((uint32_t)a)); }

int32_t s_to_f_bits(int32_t a) { return f32_to_bits((float)a); }
int32_t f_bits_to_s(int32_t a) { return (int32_t)bits_to_f32(a); }
int32_t q16_16_to_f_bits(int32_t a) { return f32_to_bits(((float)a) / 65536.0f); }
int32_t f_bits_to_q16_16(int32_t a) { return (int32_t)(bits_to_f32(a) * 65536.0f); }

int32_t fround_i32_bits(int32_t a) {
  float x = bits_to_f32(a);
  if (x >= 0.0f) return (int32_t)(x + 0.5f);
  return (int32_t)(x - 0.5f);
}


int32_t kp_fabs_f32_bits(int32_t a) { return f32_to_bits(fabsf(bits_to_f32(a))); }
int32_t kp_fsqrt_f32_bits(int32_t a) { return f32_to_bits(sqrtf(bits_to_f32(a))); }
int32_t kp_fsin_f32_bits(int32_t a) { return f32_to_bits(sinf(bits_to_f32(a))); }
int32_t kp_fcos_f32_bits(int32_t a) { return f32_to_bits(cosf(bits_to_f32(a))); }
int32_t kp_fpow_f32_i32_bits(int32_t a, int32_t n) {
  return f32_to_bits(powf(bits_to_f32(a), (float)n));
}
int32_t kp_ffloor_f32_bits(int32_t a) { return f32_to_bits(floorf(bits_to_f32(a))); }
int32_t kp_fceil_f32_bits(int32_t a) { return f32_to_bits(ceilf(bits_to_f32(a))); }

static int32_t kp_fix_from_float(float x) {
  if (x >= 0.0f) return (int32_t)(x * KP_FIX_SCALE_F + 0.5f);
  return (int32_t)(x * KP_FIX_SCALE_F - 0.5f);
}

static float kp_fix_to_float(int32_t x) {
  return ((float)x) / KP_FIX_SCALE_F;
}

int32_t kp_fx_sqrt_i32(int32_t x) {
  if (x <= 0) return 0;
  return (int32_t)floorf(sqrtf((float)x));
}

int32_t kp_fx_sin_deg_i32(int32_t a) {
  return kp_fix_from_float(sinf(((float)a) * KP_DEG2RAD_F));
}

int32_t kp_fx_cos_deg_i32(int32_t a) {
  return kp_fix_from_float(cosf(((float)a) * KP_DEG2RAD_F));
}

int32_t kp_fx_tan_deg_i32(int32_t a) {
  float c = cosf(((float)a) * KP_DEG2RAD_F);
  if (fabsf(c) < 1.0e-6f) return 0;
  return kp_fix_from_float(tanf(((float)a) * KP_DEG2RAD_F));
}

int32_t kp_fx_asin_fixed_i32(int32_t v) {
  float x = kp_fix_to_float(v);
  if (x > 1.0f) x = 1.0f;
  if (x < -1.0f) x = -1.0f;
  return (int32_t)((asinf(x) * KP_RAD2DEG_F) >= 0.0f
      ? asinf(x) * KP_RAD2DEG_F + 0.5f
      : asinf(x) * KP_RAD2DEG_F - 0.5f);
}

int32_t kp_fx_acos_fixed_i32(int32_t v) {
  float x = kp_fix_to_float(v);
  if (x > 1.0f) x = 1.0f;
  if (x < -1.0f) x = -1.0f;
  return (int32_t)((acosf(x) * KP_RAD2DEG_F) >= 0.0f
      ? acosf(x) * KP_RAD2DEG_F + 0.5f
      : acosf(x) * KP_RAD2DEG_F - 0.5f);
}

int32_t kp_fx_atan_fixed_i32(int32_t v) {
  float x = kp_fix_to_float(v);
  float deg = atanf(x) * KP_RAD2DEG_F;
  return (int32_t)(deg >= 0.0f ? deg + 0.5f : deg - 0.5f);
}

int32_t kp_fx_ln_i32(int32_t x) {
  if (x <= 0) return 0;
  return kp_fix_from_float(logf((float)x));
}

int32_t kp_fx_log_i32(int32_t x) {
  if (x <= 0) return 0;
  return kp_fix_from_float(log10f((float)x));
}
