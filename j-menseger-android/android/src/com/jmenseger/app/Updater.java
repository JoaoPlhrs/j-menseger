package com.jmenseger.app;

import android.app.PendingIntent;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.content.pm.PackageInfo;
import android.content.pm.PackageInstaller;
import android.net.Uri;
import android.os.Build;
import android.provider.Settings;

import org.json.JSONObject;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.security.MessageDigest;

/**
 * Atualização automática: lê o versao.json do site, baixa o APK novo,
 * confere o SHA-256 e entrega ao instalador do Android (o usuário confirma).
 */
final class Updater {
    static final String MANIFESTO = "https://j-menseger-site.vercel.app/versao.json";
    static final String ACAO = "com.jmenseger.app.UPDATE_STATUS";
    private static final long MAX_BYTES = 150L * 1024 * 1024;

    private static volatile JSONObject pacote;
    private static volatile boolean baixando;

    private Updater() {}

    private static void js(String chamada) {
        App a = App.get();
        if (a != null) a.runJs("window.JM&&JM." + chamada);
    }

    private static HttpURLConnection abrir(String url) throws Exception {
        URL u = new URL(url);
        if (!"https".equals(u.getProtocol())) throw new Exception("endereço inseguro");
        HttpURLConnection c = (HttpURLConnection) u.openConnection();
        c.setConnectTimeout(15000);
        c.setReadTimeout(30000);
        c.setInstanceFollowRedirects(true);
        c.setRequestProperty("User-Agent", "J-Menseger");
        return c;
    }

    private static long codigoInstalado(Context ctx) {
        try {
            PackageInfo p = ctx.getPackageManager().getPackageInfo(ctx.getPackageName(), 0);
            return Build.VERSION.SDK_INT >= 28 ? p.getLongVersionCode() : p.versionCode;
        } catch (Exception e) {
            return Long.MAX_VALUE;
        }
    }

    /** Procura uma versão nova em segundo plano; responde em JM.onUpdate(info|null). */
    static void procurar(final Context ctx) {
        new Thread(() -> {
            String res = "null";
            try {
                HttpURLConnection c = abrir(MANIFESTO);
                if (c.getResponseCode() != 200) throw new Exception("HTTP " + c.getResponseCode());
                InputStream in = c.getInputStream();
                ByteArrayOutputStream buf = new ByteArrayOutputStream();
                byte[] b = new byte[8192];
                int n;
                while ((n = in.read(b)) > 0 && buf.size() < 65536) buf.write(b, 0, n);
                in.close();
                JSONObject and = new JSONObject(buf.toString("UTF-8")).optJSONObject("android");
                if (and != null && and.optLong("codigo", 0) > codigoInstalado(ctx)
                        && and.optString("url").startsWith("https://")
                        && and.optString("sha256").length() == 64) {
                    pacote = and;
                    JSONObject info = new JSONObject();
                    info.put("versao", and.optString("versao"));
                    info.put("novidades", and.optString("novidades"));
                    info.put("tamanho", and.optLong("tamanho", 0));
                    res = info.toString();
                } else {
                    pacote = null;
                }
            } catch (Exception e) {
                res = "null";
            }
            js("onUpdate&&JM.onUpdate(" + res + ")");
        }, "jm-update").start();
    }

    /** O Android deixa este app instalar APKs? Se não, abre a tela de permissão. */
    static boolean podeInstalar(Context ctx) {
        if (ctx.getPackageManager().canRequestPackageInstalls()) return true;
        try {
            Intent i = new Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES, Uri.parse("package:" + ctx.getPackageName()));
            i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            ctx.startActivity(i);
        } catch (Exception ignored) {
        }
        return false;
    }

    /** Baixa e instala; progresso em JM.onUpdateProgress(0..100), erros em JM.onUpdateError(msg). */
    static void baixar(final Context ctx) {
        final JSONObject p = pacote;
        if (p == null || baixando) return;
        if (!podeInstalar(ctx)) {
            js("onUpdateError&&JM.onUpdateError(" + App.jsString("Permita instalar apps do J-Menseger e toque em Atualizar de novo.") + ")");
            return;
        }
        baixando = true;
        new Thread(() -> {
            File f = new File(ctx.getCacheDir(), "atualizacao.apk");
            try {
                HttpURLConnection c = abrir(p.getString("url"));
                if (c.getResponseCode() != 200) throw new Exception("HTTP " + c.getResponseCode());
                long total = c.getContentLengthLong();
                if (total > MAX_BYTES) throw new Exception("arquivo grande demais");
                MessageDigest sha = MessageDigest.getInstance("SHA-256");
                InputStream in = c.getInputStream();
                OutputStream out = new FileOutputStream(f);
                byte[] b = new byte[65536];
                long lido = 0;
                int n, ultimo = -1;
                while ((n = in.read(b)) > 0) {
                    lido += n;
                    if (lido > MAX_BYTES) throw new Exception("arquivo grande demais");
                    sha.update(b, 0, n);
                    out.write(b, 0, n);
                    if (total > 0) {
                        int pct = (int) (lido * 100 / total);
                        if (pct != ultimo) {
                            ultimo = pct;
                            js("onUpdateProgress&&JM.onUpdateProgress(" + pct + ")");
                        }
                    }
                }
                out.close();
                in.close();
                StringBuilder hex = new StringBuilder();
                for (byte x : sha.digest()) hex.append(String.format("%02x", x));
                if (!hex.toString().equalsIgnoreCase(p.getString("sha256"))) throw new Exception("arquivo corrompido (SHA-256 não confere)");
                instalar(ctx, f);
            } catch (Exception e) {
                f.delete();
                js("onUpdateError&&JM.onUpdateError(" + App.jsString("Falha ao atualizar: " + e.getMessage()) + ")");
            } finally {
                baixando = false;
            }
        }, "jm-update-dl").start();
    }

    private static void instalar(Context ctx, File apk) throws Exception {
        PackageInstaller pi = ctx.getPackageManager().getPackageInstaller();
        PackageInstaller.SessionParams sp = new PackageInstaller.SessionParams(PackageInstaller.SessionParams.MODE_FULL_INSTALL);
        sp.setAppPackageName(ctx.getPackageName());
        if (Build.VERSION.SDK_INT >= 31) sp.setRequireUserAction(PackageInstaller.SessionParams.USER_ACTION_NOT_REQUIRED);
        int id = pi.createSession(sp);
        PackageInstaller.Session s = pi.openSession(id);
        try {
            OutputStream out = s.openWrite("jm.apk", 0, apk.length());
            InputStream in = new FileInputStream(apk);
            byte[] b = new byte[65536];
            int n;
            while ((n = in.read(b)) > 0) out.write(b, 0, n);
            in.close();
            s.fsync(out);
            out.close();
            Intent st = new Intent(ACAO).setPackage(ctx.getPackageName());
            int fl = PendingIntent.FLAG_UPDATE_CURRENT | (Build.VERSION.SDK_INT >= 31 ? PendingIntent.FLAG_MUTABLE : 0);
            PendingIntent pend = PendingIntent.getBroadcast(ctx, id, st, fl);
            js("onUpdateProgress&&JM.onUpdateProgress(100)");
            s.commit(pend.getIntentSender());
        } finally {
            s.close();
            apk.delete();
        }
    }

    /** Recebe o andamento da instalação: pede a confirmação do usuário quando o Android exigir. */
    public static final class Status extends BroadcastReceiver {
        @Override
        public void onReceive(Context ctx, Intent i) {
            int st = i.getIntExtra(PackageInstaller.EXTRA_STATUS, PackageInstaller.STATUS_FAILURE);
            if (st == PackageInstaller.STATUS_PENDING_USER_ACTION) {
                Intent conf = (Intent) i.getParcelableExtra(Intent.EXTRA_INTENT);
                if (conf != null) {
                    conf.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
                    try {
                        ctx.startActivity(conf);
                    } catch (Exception ignored) {
                    }
                }
            } else if (st != PackageInstaller.STATUS_SUCCESS) {
                String m = i.getStringExtra(PackageInstaller.EXTRA_STATUS_MESSAGE);
                js("onUpdateError&&JM.onUpdateError(" + App.jsString("A instalação não terminou" + (m == null ? "." : ": " + m)) + ")");
            }
        }
    }
}
