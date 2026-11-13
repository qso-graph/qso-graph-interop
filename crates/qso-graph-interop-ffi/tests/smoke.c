/* SPDX-License-Identifier: GPL-3.0-or-later */
/* Independent foreign caller: compile with cc and link the actual static library. */
#include "qso_graph_interop.h"
#include <assert.h>
#include <stdio.h>
#include <string.h>

static int contains(QgiBuffer b, const char *text) {
    size_t n = strlen(text);
    for (size_t i = 0; i + n <= b.len; ++i) {
        if (memcmp(b.data + i, text, n) == 0) return 1;
    }
    return 0;
}
static QgiCodec codec(const char *family) {
    QgiCodec h = 0;
    assert(qgi_codec_new((const uint8_t *)family, strlen(family), &h) == QGI_OK);
    assert(h != 0);
    return h;
}
static void observe(QgiCodec h, const uint8_t *bytes, size_t len,
                    const char *options, const char *family) {
    QgiBuffer out = {0};
    assert(qgi_decode(h, bytes, len, (const uint8_t *)options, strlen(options), &out) == QGI_OK);
    assert(contains(out, family));
    assert(contains(out, "\"events\":[{"));
    assert(contains(out, "\"skips\":[]"));
    assert(qgi_buffer_free(out) == QGI_OK);
    assert(qgi_buffer_free(out) == QGI_BAD_BUFFER);
}
int main(void) {
    assert(qgi_abi_version() == QGI_ABI_VERSION);
    QgiBuffer out = {0};
    assert(qgi_version(&out) == QGI_OK);
    assert(out.len > 0);
    assert(qgi_buffer_free(out) == QGI_OK);
    assert(qgi_version(NULL) == QGI_NULL_POINTER);
    QgiError err = qgi_last_error();
    assert(err.data != NULL && err.len != 0);
    assert(qgi_last_error().data == err.data);

    QgiCodec w = codec("wsjtx");
    /* WSJT-X schema 3 HaltTx, empty wire ID, auto_tx_only=true. */
    const uint8_t datagram[] = {0xad,0xbc,0xcb,0xda,0,0,0,3,0,0,0,8,0,0,0,0,1};
    observe(w, datagram, sizeof datagram, "{}", "\"family\":\"wsjtx\"");
    assert(qgi_decode(w, NULL, 0, NULL, 0, &out) == QGI_TRUNCATED);
    assert(qgi_decode(w, NULL, 1, NULL, 0, &out) == QGI_NULL_POINTER);
    assert(qgi_decode(w, NULL, SIZE_MAX, NULL, 0, &out) == QGI_BUFFER_TOO_LARGE);
    assert(qgi_decode(0, NULL, 0, NULL, 0, &out) == QGI_BAD_HANDLE);
    const char command[] = "{\"spec\":\"0.1.0\",\"family\":\"wsjtx\",\"mapping\":1,\"kind\":\"control\",\"wire_id\":\"\",\"context\":{\"direction\":\"to_application\",\"exchange\":\"request\"},\"fields\":{\"operation\":\"halt_tx\",\"auto_tx_only\":true}}";
    size_t count = 0;
    assert(qgi_encode_command(w, (const uint8_t *)command, strlen(command), NULL, 0, 0, &count, &out) == QGI_OK);
    assert(count == 1 && out.len == sizeof datagram);
    assert(memcmp(out.data, datagram, sizeof datagram) == 0);
    assert(qgi_codec_free(w) == QGI_OK);
    /* Output lifetime is independent of the handle. */
    assert(qgi_buffer_free(out) == QGI_OK);
    assert(qgi_codec_free(w) == QGI_BAD_HANDLE);

    QgiCodec n = codec("n1mm");
    const char radio[] = "<RadioInfo><StationName>Q-SYNTHETIC</StationName><RadioNr>1</RadioNr><Freq>1407400</Freq></RadioInfo>";
    observe(n, (const uint8_t *)radio, strlen(radio), "{}", "\"family\":\"n1mm\"");
    assert(qgi_reset(n, NULL, 0) == QGI_OK);
    assert(qgi_codec_free(n) == QGI_OK);

    QgiCodec f = codec("fldigi");
    const char response[] = "<methodResponse><params><param><value><double>14074000.0</double></value></param></params></methodResponse>";
    observe(f, (const uint8_t *)response, strlen(response), "{\"native\":{\"request\":{\"method\":\"rig.get_frequency\"}},\"context\":{\"direction\":\"from_application\",\"exchange\":\"response\"}}", "\"family\":\"fldigi\"");
    const char text[] = "{\"spec\":\"0.1.0\",\"family\":\"fldigi\",\"mapping\":1,\"kind\":\"control\",\"context\":{\"direction\":\"to_application\",\"exchange\":\"request\"},\"fields\":{\"operation\":\"send_text\",\"text\":\"CQ Q1AAA\",\"transmit\":true}}";
    for (size_t i = 0; i < 2; ++i) {
        assert(qgi_encode_command(f, (const uint8_t *)text, strlen(text), NULL, 0, i, &count, &out) == QGI_OK);
        assert(count == 2);
        assert(contains(out, i == 0 ? "text.add_tx" : "main.tx"));
        assert(qgi_buffer_free(out) == QGI_OK);
    }
    assert(qgi_encode_command(f, (const uint8_t *)text, strlen(text), NULL, 0, 2, &count, &out) == QGI_INVALID_VALUE);
    assert(out.token == 0 && count == 0);
    assert(qgi_codec_free(f) == QGI_OK);
    assert(qgi_buffer_free((QgiBuffer){0}) == QGI_OK);
    puts("C ABI: WSJT-X, N1MM, fldigi, commands, statuses and ownership passed");
    return 0;
}
