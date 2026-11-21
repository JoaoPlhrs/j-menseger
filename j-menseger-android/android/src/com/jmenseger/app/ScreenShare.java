package com.jmenseger.app;

import android.app.Activity;
import android.content.Context;
import android.content.Intent;
import android.graphics.Bitmap;
import android.graphics.PixelFormat;
import android.hardware.display.DisplayManager;
import android.hardware.display.VirtualDisplay;
import android.media.Image;
import android.media.ImageReader;
import android.media.projection.MediaProjection;
import android.media.projection.MediaProjectionManager;
import android.os.Handler;
import android.os.HandlerThread;
import android.util.DisplayMetrics;

import java.io.ByteArrayOutputStream;
import java.nio.ByteBuffer;

/**
 * Transmissão da tela do celular.
 *
 * O Android pede permissão (a janela "Começar a transmitir?"), o serviço em
 * primeiro plano passa a ser do tipo "projeção de mídia" e uma tela virtual
 * copia a imagem para um ImageReader. Cada quadro vira um JPEG pequeno que a
 * página busca em /__screen.jpg e desenha num canvas — e o canvas vira a
 * trilha de vídeo da chamada, como a câmera.
 */
final class ScreenShare {
    static final int REQ = 21;
    /** Lado maior da imagem enviada (720p no celular em pé = 720×1600 cortado para 1280). */
    private static final int MAX_LADO = 1280;
    /** No máximo ~12 quadros por segundo: suficiente para mostrar a tela e leve para o celular. */
    private static final long INTERVALO_MS = 80;

    static volatile byte[] latest;
    static volatile boolean running;

    private static MediaProjection projection;
    private static VirtualDisplay display;
    private static ImageReader reader;
    private static HandlerThread thread;
    private static long ultimo;

    private ScreenShare() {}

    /** Mostra a janela de permissão do Android. A resposta chega em onResult. */
    static void ask(Activity a) {
        MediaProjectionManager m = (MediaProjectionManager) a.getSystemService(Context.MEDIA_PROJECTION_SERVICE);
        if (m == null) {
            report(false);
            return;
        }
        try {
            a.startActivityForResult(m.createScreenCaptureIntent(), REQ);
        } catch (Exception e) {
            report(false);
        }
    }

    static void onResult(Activity a, int res, Intent data) {
        if (res != Activity.RESULT_OK || data == null) {
            report(false);
            return;
        }
        // Android 14: o serviço tem de estar em primeiro plano como "projeção de mídia" antes de pegar a captura.
        ConnService.screen = true;
        ConnService.sync(a);
        final Context app = a.getApplicationContext();
        final int r = res;
        final Intent d = data;
        new Handler(a.getMainLooper()).postDelayed(() -> start(app, r, d), 400);
    }

    private static void start(Context ctx, int res, Intent data) {
        try {
            MediaProjectionManager m = (MediaProjectionManager) ctx.getSystemService(Context.MEDIA_PROJECTION_SERVICE);
            projection = m.getMediaProjection(res, data);
            if (projection == null) throw new IllegalStateException("sem projeção");
            thread = new HandlerThread("jm-tela");
            thread.start();
            Handler h = new Handler(thread.getLooper());
            projection.registerCallback(new MediaProjection.Callback() {
                @Override
                public void onStop() {
                    stop();
                }
            }, h);
            DisplayMetrics dm = ctx.getResources().getDisplayMetrics();
            int w = dm.widthPixels, hgt = dm.heightPixels;
            float esc = Math.min(1f, MAX_LADO / (float) Math.max(w, hgt));
            int vw = Math.max(2, Math.round(w * esc) & ~1), vh = Math.max(2, Math.round(hgt * esc) & ~1);
            reader = ImageReader.newInstance(vw, vh, PixelFormat.RGBA_8888, 2);
            reader.setOnImageAvailableListener(rd -> {
                Image img = null;
                try {
                    img = rd.acquireLatestImage();
                    if (img == null) return;
                    long agora = System.currentTimeMillis();
                    if (agora - ultimo < INTERVALO_MS) return;
                    ultimo = agora;
                    latest = jpeg(img);
                } catch (Exception ignored) {
                } finally {
                    if (img != null) img.close();
                }
            }, h);
            display = projection.createVirtualDisplay("J-Menseger", vw, vh, dm.densityDpi, DisplayManager.VIRTUAL_DISPLAY_FLAG_AUTO_MIRROR, reader.getSurface(), null, h);
            running = true;
            report(true);
        } catch (Exception e) {
            stop();
        }
    }

    private static byte[] jpeg(Image img) {
        Image.Plane p = img.getPlanes()[0];
        ByteBuffer buf = p.getBuffer();
        int w = img.getWidth(), h = img.getHeight();
        int pixel = p.getPixelStride(), linha = p.getRowStride();
        int sobra = linha - pixel * w;
        Bitmap bmp = Bitmap.createBitmap(w + sobra / pixel, h, Bitmap.Config.ARGB_8888);
        bmp.copyPixelsFromBuffer(buf);
        Bitmap certo = sobra > 0 ? Bitmap.createBitmap(bmp, 0, 0, w, h) : bmp;
        ByteArrayOutputStream out = new ByteArrayOutputStream(64 * 1024);
        certo.compress(Bitmap.CompressFormat.JPEG, 62, out);
        if (certo != bmp) certo.recycle();
        bmp.recycle();
        return out.toByteArray();
    }

    static synchronized void stop() {
        boolean estava = running;
        running = false;
        latest = null;
        try {
            if (display != null) display.release();
        } catch (Exception ignored) {
        }
        try {
            if (reader != null) reader.close();
        } catch (Exception ignored) {
        }
        try {
            if (projection != null) projection.stop();
        } catch (Exception ignored) {
        }
        if (thread != null) thread.quitSafely();
        display = null;
        reader = null;
        projection = null;
        thread = null;
        if (ConnService.screen) {
            ConnService.screen = false;
            App a = App.get();
            if (a != null) ConnService.sync(a);
        }
        // Também quando nem chegou a começar: a página volta o botão ao normal.
        if (estava || ConnService.running) report(false);
    }

    private static void report(boolean on) {
        App a = App.get();
        if (a != null) a.runJs("window.JM&&JM.onScreen&&JM.onScreen(" + on + ")");
    }
}
