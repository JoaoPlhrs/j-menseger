package com.jmenseger.app;

import android.graphics.Bitmap;
import android.graphics.BitmapFactory;
import android.util.Base64;

import org.json.JSONObject;

import java.io.ByteArrayOutputStream;
import java.io.InputStream;
import java.net.HttpURLConnection;
import java.net.InetAddress;
import java.net.URL;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * Prévia de link feita por quem manda: só este aparelho acessa o site; o
 * título, a descrição e uma miniatura vão cifrados junto com a mensagem.
 * Responde em JM.onLinkPreview(id, json|null).
 */
final class LinkPreview {
    private LinkPreview() {}

    static void fetch(final String id, final String url) {
        new Thread(() -> {
            String res = "null";
            try {
                JSONObject o = build(url);
                if (o != null) res = o.toString();
            } catch (Throwable ignored) {
            }
            App a = App.get();
            if (a != null) a.runJs("window.JM&&JM.onLinkPreview&&JM.onLinkPreview(" + App.jsString(id) + "," + res + ")");
        }, "jm-link").start();
    }

    /** Endereço da rede de casa ou do próprio aparelho: nunca acessar. */
    private static boolean privado(URL u) {
        try {
            InetAddress ip = InetAddress.getByName(u.getHost());
            return ip.isLoopbackAddress() || ip.isSiteLocalAddress() || ip.isLinkLocalAddress() || ip.isAnyLocalAddress();
        } catch (Exception e) {
            return true;
        }
    }

    private static HttpURLConnection abrir(String url) throws Exception {
        URL u = new URL(url);
        String p = u.getProtocol();
        if ((!"https".equals(p) && !"http".equals(p)) || privado(u)) throw new Exception("bloqueado");
        HttpURLConnection c = (HttpURLConnection) u.openConnection();
        c.setConnectTimeout(4000);
        c.setReadTimeout(5000);
        c.setInstanceFollowRedirects(true);
        c.setRequestProperty("User-Agent", "Mozilla/5.0 (compatible; J-Menseger link preview)");
        return c;
    }

    private static byte[] ler(InputStream in, int max) throws Exception {
        ByteArrayOutputStream b = new ByteArrayOutputStream();
        byte[] buf = new byte[16384];
        int n;
        while ((n = in.read(buf)) > 0 && b.size() < max) b.write(buf, 0, n);
        in.close();
        return b.toByteArray();
    }

    private static String meta(String html, String chave) {
        Pattern p = Pattern.compile("<meta[^>]+(?:property|name)=[\"']" + Pattern.quote(chave) + "[\"'][^>]*>", Pattern.CASE_INSENSITIVE);
        Matcher m = p.matcher(html);
        while (m.find()) {
            Matcher c = Pattern.compile("content=[\"']([^\"']*)[\"']", Pattern.CASE_INSENSITIVE).matcher(m.group());
            if (c.find() && !c.group(1).trim().isEmpty()) return desesc(c.group(1).trim());
        }
        return null;
    }

    private static String desesc(String s) {
        return s.replace("&amp;", "&").replace("&quot;", "\"").replace("&#39;", "'").replace("&lt;", "<").replace("&gt;", ">").replace("&nbsp;", " ");
    }

    private static JSONObject build(String url) throws Exception {
        HttpURLConnection c = abrir(url);
        String tipo = c.getContentType();
        if (c.getResponseCode() != 200 || tipo == null || !tipo.toLowerCase().contains("html")) return null;
        String html = new String(ler(c.getInputStream(), 600000), "UTF-8");
        String t = meta(html, "og:title");
        if (t == null) t = meta(html, "twitter:title");
        if (t == null) {
            Matcher m = Pattern.compile("<title[^>]*>([^<]*)</title>", Pattern.CASE_INSENSITIVE).matcher(html);
            if (m.find()) t = desesc(m.group(1).trim());
        }
        if (t == null || t.isEmpty()) return null;
        String d = meta(html, "og:description");
        if (d == null) d = meta(html, "description");
        JSONObject o = new JSONObject();
        o.put("u", url);
        o.put("t", t.length() > 140 ? t.substring(0, 140) : t);
        if (d != null) o.put("d", d.length() > 240 ? d.substring(0, 240) : d);
        String img = meta(html, "og:image");
        if (img != null) {
            try {
                URL abs = new URL(c.getURL(), img);
                byte[] b = ler(abrir(abs.toString()).getInputStream(), 1500000);
                String i = thumb(b);
                if (i != null) o.put("i", i);
            } catch (Exception ignored) {
            }
        }
        return o;
    }

    private static String thumb(byte[] b) {
        Bitmap bmp = BitmapFactory.decodeByteArray(b, 0, b.length);
        if (bmp == null) return null;
        float lado = 200f;
        while (lado >= 40f) {
            float e = Math.min(1f, lado / Math.max(bmp.getWidth(), bmp.getHeight()));
            Bitmap p = Bitmap.createScaledBitmap(bmp, Math.max(1, Math.round(bmp.getWidth() * e)), Math.max(1, Math.round(bmp.getHeight() * e)), true);
            for (int q : new int[]{70, 55, 40}) {
                ByteArrayOutputStream out = new ByteArrayOutputStream();
                p.compress(Bitmap.CompressFormat.JPEG, q, out);
                String u = "data:image/jpeg;base64," + Base64.encodeToString(out.toByteArray(), Base64.NO_WRAP);
                if (u.length() <= 24000) return u;
            }
            lado *= 0.7f;
        }
        return null;
    }
}
