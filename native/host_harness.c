// SPDX-License-Identifier: GPL-2.0-or-later
// Run the exact platform bridge against a read-only pack before device play.
#include <stdint.h>
#include <string.h>
#include <setjmp.h>
static const uint8_t *image;
static uint32_t image_size,clock_ms,buttons;
static uint16_t screen[320*240];
static char error[192];
static jmp_buf bail_out;
void km_doom_init(void);
void km_doom_step(void);
uint32_t km_millis(void) { return clock_ms; }
void km_sleep(uint32_t ms) { clock_ms+=ms; }
uint32_t km_keys(void) { return buttons; }
__attribute__((noreturn)) void km_fail(const char *message) {
    size_t n=strlen(message);if(n>=sizeof(error))n=sizeof(error)-1;
    memcpy(error,message,n);error[n]=0;longjmp(bail_out,1);
}
__attribute__((noreturn)) void km_exit(void) { km_fail("Game requested exit"); }
void km_flash_read(uint32_t off,void *dst,uint32_t n) {
    if(off>image_size || n>image_size-off)km_fail("Host pack read out of range");
    memcpy(dst,image+off,n);
}
void km_column(int x,int y,int h,const uint16_t *pixels) {
    if(x<0 || x>=320 || y<0 || h<0 || y+h>240)km_fail("Host LCD bounds");
    for(int i=0;i<h;i++)screen[(y+i)*320+x]=pixels[i];
}
__declspec(dllexport) int km_host_init(const uint8_t *data,uint32_t n) {
    image=data;image_size=n;clock_ms=0;
    if(setjmp(bail_out))return -1;km_doom_init();return 0;
}
__declspec(dllexport) int km_host_step(uint32_t keys) {
    clock_ms+=29;buttons=keys;
    if(setjmp(bail_out))return -1;km_doom_step();return 0;
}
__declspec(dllexport) const char *km_host_error(void) { return error; }
__declspec(dllexport) const uint16_t *km_host_screen(void) { return screen; }
