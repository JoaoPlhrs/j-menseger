/* As versões de ABI do libvpx mudam entre versões da biblioteca. Fixá-las no
 * Rust daria erro silencioso em outro sistema, então elas são lidas dos
 * cabeçalhos na hora de compilar. */
#include <vpx/vpx_encoder.h>
#include <vpx/vpx_decoder.h>
#include <vpx/vp8cx.h>
#include <vpx/vp8dx.h>

int jm_vpx_encoder_abi_version(void) { return VPX_ENCODER_ABI_VERSION; }
int jm_vpx_decoder_abi_version(void) { return VPX_DECODER_ABI_VERSION; }
