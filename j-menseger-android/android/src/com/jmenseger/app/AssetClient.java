package com.jmenseger.app;

import android.content.Context;
import android.content.Intent;
import android.net.Uri;
import android.os.Handler;
import android.os.Looper;
import android.webkit.RenderProcessGoneDetail;
import android.webkit.WebResourceRequest;
import android.webkit.WebResourceResponse;
import android.webkit.WebView;
import android.webkit.WebViewClient;

import java.io.ByteArrayInputStream;
import java.io.InputStream;
import java.util.HashMap;
import java.util.Map;

/**
 * Serve a página de dentro do APK (assets/www) num endereço https próprio e
 * bloqueia qualquer outro endereço web. Exceção: imagens e busca do GIPHY.
 * Os servidores de mensagens (wss://) não passam por aqui.
 */
public class AssetClient extends WebViewClient {
    private final Context ctx;

    AssetClient(Context ctx) {
        this.ctx = ctx;
    }

    static boolean allowedRemote(Uri u) {
        if (!"https".equals(u.getScheme())) return false;
        String h = u.getHost();
        if (h == null) return false;
        h = h.toLowerCase();
        return h.equals("api.giphy.com") || h.equals("giphy.com") || h.endsWith(".giphy.com");
    }

    static String mime(String path) {
        String p = path.toLowerCase();
        if (p.endsWith(".html")) return "text/html";
        if (p.endsWith(".js")) return "text/javascript";
        if (p.endsWith(".css")) return "text/css";
        if (p.endsWith(".json")) return "application/json";
        if (p.endsWith(".svg")) return "image/svg+xml";
        if (p.endsWith(".png")) return "image/png";
        if (p.endsWith(".ogg")) return "audio/ogg";
        if (p.endsWith(".woff2")) return "font/woff2";
        if (p.endsWith(".txt")) return "text/plain";
        return "application/octet-stream";
    }

    private WebResourceResponse blocked(int code, String msg) {
        WebResourceResponse r = new WebResourceResponse("text/plain", "utf-8", new ByteArrayInputStream(new byte[0]));
        r.setStatusCodeAndReasonPhrase(code, msg);
        return r;
    }

    @Override
    public WebResourceResponse shouldInterceptRequest(WebView view, WebResourceRequest req) {
        Uri u = req.getUrl();
        if (!"appassets.androidplatform.net".equals(u.getHost())) {
            if (allowedRemote(u)) return null;
            return blocked(403, "Forbidden");
        }
        String path = u.getPath();
        if (path == null || path.equals("/")) path = "/index.html";
        if (path.contains("..")) return blocked(404, "Not Found");
        try {
            InputStream in = ctx.getAssets().open("www" + path);
            String m = mime(path);
            WebResourceResponse r = new WebResourceResponse(m, m.startsWith("text/") || m.equals("application/json") ? "utf-8" : null, in);
            Map<String, String> h = new HashMap<String, String>();
            h.put("Cache-Control", "no-cache");
            h.put("X-Content-Type-Options", "nosniff");
            h.put("Referrer-Policy", "no-referrer");
            r.setResponseHeaders(h);
            return r;
        } catch (Exception e) {
            return blocked(404, "Not Found");
        }
    }

    @Override
    public boolean shouldOverrideUrlLoading(WebView view, WebResourceRequest req) {
        Uri u = req.getUrl();
        if ("appassets.androidplatform.net".equals(u.getHost())) return false;
        String s = u.getScheme();
        if ("http".equals(s) || "https".equals(s)) {
            try {
                Intent i = new Intent(Intent.ACTION_VIEW, u).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
                ctx.startActivity(i);
            } catch (Exception ignored) {
            }
        }
        return true;
    }

    @Override
    public void onPageStarted(WebView view, String url, android.graphics.Bitmap favicon) {
        super.onPageStarted(view, url, favicon);
    }

    @Override
    public void onPageFinished(WebView view, String url) {
        super.onPageFinished(view, url);
        App.get().onPageReady();
    }

    /** O motor da página caiu (falta de memória): monta tudo de novo. */
    @Override
    public boolean onRenderProcessGone(WebView view, RenderProcessGoneDetail detail) {
        App.get().destroyWeb();
        new Handler(Looper.getMainLooper()).post(() -> {
                MainActivity a = MainActivity.current;
                if (a != null) a.attachWeb();
            });
        return true;
    }
}
