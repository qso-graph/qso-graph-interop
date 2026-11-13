// SPDX-License-Identifier: GPL-3.0-or-later
#include "qso_graph_interop.h"
#include <type_traits>
static_assert(std::is_standard_layout<QgiBuffer>::value, "C buffer layout");
static_assert(QGI_ABI_VERSION == 1, "expected ABI");
int main() {
    QgiCodec h = 0;
    QgiBuffer out = {};
    const uint8_t name[] = {'w','s','j','t','x'};
    int32_t status = qgi_codec_new(name, sizeof name, &h);
    status += qgi_decode(h, nullptr, 0, nullptr, 0, &out);
    status += qgi_buffer_free(out);
    return status + qgi_codec_free(h);
}
