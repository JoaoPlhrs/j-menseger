package com.jmenseger.app;

import android.app.Application;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.content.MutableContextWrapper;
import android.os.Build;
import android.os.Handler;
import android.os.Looper;
import android.webkit.WebSettings;
import android.webkit.WebView;

import java.util.ArrayList;
import java.util.List;

/**
 * O aplicativo. Guarda uma única WebView que sobrevive à tela: fechar a tela
 * (voltar) não derruba a conexão com os servidores enquanto o serviço em
 * segundo plano estiver ligado.
 */
public class App extends Application {
    static final String ORIGIN = "https://appassets.androidplatform.net";
    static final String START_URL = ORIGIN + "/index.html";

    private static App instance;
    private final Handler main = new Handler(Looper.getMainLooper());
    private WebView web;
    private MutableContextWrapper wrapper;
    private boolean pageReady;
    private final List<String> pendingJs = new ArrayList<String>();

    /** Última música vista no Spotify (JSON), ou null. */
    static volatile String nowPlaying;

    static App get() {
        return instance;
    }

    @Override
    public void onCreate() {
        super.onCreate();
        instance = this;
        Notifier.channels(this);
        listenSpotify();
    }

    /** A WebView (cria na primeira vez). `ctx` é a tela que vai mostrá-la. */
    WebView web(Context ctx) {
        if (web == null) {
            wrapper = new MutableContextWrapper(ctx);
            web = new WebView(wrapper);
            WebSettings s = web.getSettings();
            s.setJavaScriptEnabled(true);
            s.setDomStorageEnabled(true);
            s.setDatabaseEnabled(true);
            s.setMediaPlaybackRequiresUserGesture(false);
            s.setAllowFileAccess(false);
            s.setAllowContentAccess(false);
            s.setGeolocationEnabled(false);
            s.setSupportZoom(false);
            s.setBuiltInZoomControls(false);
            s.setTextZoom(100);
            s.setSaveFormData(false);
            s.setMixedContentMode(WebSettings.MIXED_CONTENT_NEVER_ALLOW);
            s.setCacheMode(WebSettings.LOAD_NO_CACHE);
            web.setBackgroundColor(0xFF000000);
            web.setOverScrollMode(WebView.OVER_SCROLL_NEVER);
            web.addJavascriptInterface(new Bridge(this), "JMNative");
            web.setWebViewClient(new AssetClient(this));
            web.setWebChromeClient(new ChromeClient());
            pageReady = false;
            web.loadUrl(START_URL);
        } else {
            rebase(ctx);
        }
        return web;
    }

    boolean hasWeb() {
        return web != null;
    }

    WebView peekWeb() {
        return web;
    }

    void rebase(Context ctx) {
        if (wrapper != null) wrapper.setBaseContext(ctx);
    }

    void destroyWeb() {
        if (web != null) {
            try {
                if (web.getParent() instanceof android.view.ViewGroup) ((android.view.ViewGroup) web.getParent()).removeView(web);
                web.destroy();
            } catch (Throwable ignored) {
            }
        }
        web = null;
        pageReady = false;
        synchronized (pendingJs) {
            pendingJs.clear();
        }
    }

    void onPageReady() {
        pageReady = true;
        List<String> fila;
        synchronized (pendingJs) {
            fila = new ArrayList<String>(pendingJs);
            pendingJs.clear();
        }
        for (String js : fila) runJs(js);
    }

    /** Roda JavaScript na página (espera ela carregar, se preciso). */
    void runJs(final String js) {
        if (web == null || !pageReady) {
            synchronized (pendingJs) {
                pendingJs.add(js);
            }
            return;
        }
        main.post(() -> {
                if (web != null) web.evaluateJavascript(js, null);
            });
    }

    /** Texto como literal de JavaScript, seguro para colar no código. */
    static String jsString(String s) {
        if (s == null) return "null";
        StringBuilder b = new StringBuilder("\"");
        for (int i = 0; i < s.length(); i++) {
            char c = s.charAt(i);
            switch (c) {
                case '"': b.append("\\\""); break;
                case '\\': b.append("\\\\"); break;
                case '\n': b.append("\\n"); break;
                case '\r': b.append("\\r"); break;
                case '<': b.append("\\u003c"); break;
                case '>': b.append("\\u003e"); break;
                case ' ': b.append("\\u2028"); break;
                case ' ': b.append("\\u2029"); break;
                default:
                    if (c < 0x20) b.append(String.format("\\u%04x", (int) c));
                    else b.append(c);
            }
        }
        return b.append('"').toString();
    }

    /**
     * Spotify: com "Transmissão do status do dispositivo" ligada nos ajustes
     * do Spotify, ele avisa os outros apps qual música está tocando.
     */
    private void listenSpotify() {
        IntentFilter f = new IntentFilter();
        f.addAction("com.spotify.music.metadatachanged");
        f.addAction("com.spotify.music.playbackstatechanged");
        BroadcastReceiver r = new SpotifyReceiver();
        if (Build.VERSION.SDK_INT >= 33) registerReceiver(r, f, Context.RECEIVER_EXPORTED);
        else registerReceiver(r, f);
    }

    private static String str(String s) {
        return s == null ? "" : s;
    }

    /** Recebe os avisos do Spotify (classe com nome: o d8 desta versão falha com classes anônimas do JDK 21). */
    static final class SpotifyReceiver extends BroadcastReceiver {
            private String track = "", artist = "", album = "", id = "";
            private long length = 0;
            private boolean playing = false;

            @Override
            public void onReceive(Context c, Intent i) {
                String a = i.getAction();
                if ("com.spotify.music.metadatachanged".equals(a)) {
                    track = str(i.getStringExtra("track"));
                    artist = str(i.getStringExtra("artist"));
                    album = str(i.getStringExtra("album"));
                    id = str(i.getStringExtra("id"));
                    length = i.getIntExtra("length", 0);
                    if (i.hasExtra("playing")) playing = i.getBooleanExtra("playing", false);
                } else {
                    playing = i.getBooleanExtra("playing", false);
                }
                String json = playing && track.length() > 0
                        ? "{\"s\":" + jsString(track) + ",\"a\":" + jsString(artist) + ",\"al\":" + jsString(album)
                          + ",\"id\":" + jsString(id) + ",\"len\":" + length + "}"
                        : null;
                nowPlaying = json;
                App.get().runJs("window.JM&&JM.onSpotify&&JM.onSpotify(" + (json == null ? "null" : json) + ")");
            }
    }
}
