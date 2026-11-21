package com.jmenseger.app;

import android.app.Activity;
import android.app.NotificationManager;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.content.pm.PackageManager;
import android.graphics.Color;
import android.media.AudioDeviceInfo;
import android.media.AudioManager;
import android.net.Uri;
import android.os.Build;
import android.os.Handler;
import android.os.Looper;
import android.os.PowerManager;
import android.os.VibrationEffect;
import android.os.Vibrator;
import android.os.VibratorManager;
import android.provider.Settings;
import android.view.View;
import android.view.Window;
import android.view.WindowInsetsController;
import android.view.WindowManager;
import android.webkit.JavascriptInterface;

/** O que a página pode pedir ao Android (window.JMNative). */
public class Bridge {
    static final String PREFS = "jm_native";
    private final Context ctx;
    private final Handler main = new Handler(Looper.getMainLooper());
    private PowerManager.WakeLock proximity;

    Bridge(Context ctx) {
        this.ctx = ctx.getApplicationContext();
    }

    private void ui(Runnable r) {
        main.post(r);
    }

    private SharedPreferences prefs() {
        return ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE);
    }

    private static Activity act() {
        return MainActivity.current;
    }

    @JavascriptInterface
    public String version() {
        try {
            return ctx.getPackageManager().getPackageInfo(ctx.getPackageName(), 0).versionName;
        } catch (PackageManager.NameNotFoundException e) {
            return "?";
        }
    }

    /* ---------------- cofre ---------------- */

    @JavascriptInterface
    public String secureGet(String k) {
        return SecureStore.get(ctx, k);
    }

    @JavascriptInterface
    public void secureSet(String k, String v) {
        SecureStore.set(ctx, k, v);
    }

    @JavascriptInterface
    public void secureDel(String k) {
        SecureStore.del(ctx, k);
    }

    /* ---------------- avisos ---------------- */

    @JavascriptInterface
    public void notify(String json) {
        Notifier.message(ctx, json);
    }

    @JavascriptInterface
    public void cancelNotify(String tag) {
        Notifier.cancel(ctx, tag);
    }

    @JavascriptInterface
    public void incomingCall(String json) {
        Notifier.incomingCall(ctx, json);
    }

    @JavascriptInterface
    public void cancelIncoming() {
        Notifier.cancelIncoming(ctx);
    }

    @JavascriptInterface
    public boolean notificationsAllowed() {
        NotificationManager nm = (NotificationManager) ctx.getSystemService(Context.NOTIFICATION_SERVICE);
        return nm != null && nm.areNotificationsEnabled();
    }

    @JavascriptInterface
    public void requestNotifications() {
        ui(() -> {
                MainActivity a = MainActivity.current;
                if (Build.VERSION.SDK_INT >= 33 && a != null) a.askNotifications();
                else appSettings();
            });
    }

    /** Android 14+: chamada em tela cheia com o celular bloqueado precisa de permissão. */
    @JavascriptInterface
    public boolean fullScreenAllowed() {
        if (Build.VERSION.SDK_INT < 34) return true;
        NotificationManager nm = (NotificationManager) ctx.getSystemService(Context.NOTIFICATION_SERVICE);
        return nm != null && nm.canUseFullScreenIntent();
    }

    @JavascriptInterface
    public void fullScreenSettings() {
        if (Build.VERSION.SDK_INT < 34) return;
        start(new Intent(Settings.ACTION_MANAGE_APP_USE_FULL_SCREEN_INTENT, Uri.parse("package:" + ctx.getPackageName())));
    }

    /* ---------------- segundo plano e chamadas ---------------- */

    @JavascriptInterface
    public void setBackground(boolean on) {
        prefs().edit().putBoolean("bg", on).apply();
        if (on) ConnService.sync(ctx);
        else if (!ConnService.inCall) ctx.stopService(new Intent(ctx, ConnService.class));
    }

    @JavascriptInterface
    public void callMode(final boolean on, final boolean video) {
        ConnService.inCall = on;
        ConnService.callVideo = video;
        ui(() -> {
                AudioManager am = (AudioManager) ctx.getSystemService(Context.AUDIO_SERVICE);
                if (on) {
                    am.setMode(AudioManager.MODE_IN_COMMUNICATION);
                    setSpeakerInternal(video);
                    if (!video) acquireProximity();
                    else releaseProximity();
                } else {
                    releaseProximity();
                    if (Build.VERSION.SDK_INT >= 31) am.clearCommunicationDevice();
                    else am.setSpeakerphoneOn(false);
                    am.setMode(AudioManager.MODE_NORMAL);
                }
                keepScreenInternal(on);
                if (on || prefs().getBoolean("bg", false)) ConnService.sync(ctx);
                else ctx.stopService(new Intent(ctx, ConnService.class));
            });
    }

    @JavascriptInterface
    public void setSpeaker(final boolean on) {
        ui(() -> {
                setSpeakerInternal(on);
                if (on) releaseProximity();
                else if (ConnService.inCall && !ConnService.callVideo) acquireProximity();
            });
    }

    private void setSpeakerInternal(boolean on) {
        AudioManager am = (AudioManager) ctx.getSystemService(Context.AUDIO_SERVICE);
        if (Build.VERSION.SDK_INT >= 31) {
            AudioDeviceInfo alvo = null;
            for (AudioDeviceInfo d : am.getAvailableCommunicationDevices()) {
                int t = d.getType();
                if (on && t == AudioDeviceInfo.TYPE_BUILTIN_SPEAKER) alvo = d;
                if (!on && (t == AudioDeviceInfo.TYPE_BLUETOOTH_SCO || t == AudioDeviceInfo.TYPE_WIRED_HEADSET || t == AudioDeviceInfo.TYPE_USB_HEADSET || t == AudioDeviceInfo.TYPE_BLE_HEADSET)) alvo = d;
                if (!on && alvo == null && t == AudioDeviceInfo.TYPE_BUILTIN_EARPIECE) alvo = d;
            }
            if (alvo != null) am.setCommunicationDevice(alvo);
            else am.clearCommunicationDevice();
        } else {
            am.setSpeakerphoneOn(on);
        }
    }

    private void acquireProximity() {
        try {
            if (proximity == null) {
                PowerManager pm = (PowerManager) ctx.getSystemService(Context.POWER_SERVICE);
                if (!pm.isWakeLockLevelSupported(PowerManager.PROXIMITY_SCREEN_OFF_WAKE_LOCK)) return;
                proximity = pm.newWakeLock(PowerManager.PROXIMITY_SCREEN_OFF_WAKE_LOCK, "jmenseger:proximity");
            }
            if (!proximity.isHeld()) proximity.acquire(4 * 60 * 60 * 1000L);
        } catch (Exception ignored) {
        }
    }

    private void releaseProximity() {
        try {
            if (proximity != null && proximity.isHeld()) proximity.release();
        } catch (Exception ignored) {
        }
    }

    /* ---------------- janela ---------------- */

    @JavascriptInterface
    public void setSecure(final boolean on) {
        prefs().edit().putBoolean("secure", on).apply();
        ui(() -> {
                Activity a = act();
                if (a == null) return;
                if (on) a.getWindow().addFlags(WindowManager.LayoutParams.FLAG_SECURE);
                else a.getWindow().clearFlags(WindowManager.LayoutParams.FLAG_SECURE);
            });
    }

    @JavascriptInterface
    public void keepScreen(final boolean on) {
        ui(() -> {
                keepScreenInternal(on);
            });
    }

    private void keepScreenInternal(boolean on) {
        Activity a = act();
        if (a == null) return;
        if (on) a.getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
        else a.getWindow().clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
    }

    /** Cor das barras do sistema; `light` = ícones escuros (tema claro). */
    @JavascriptInterface
    public void setBars(final String bg, final boolean light) {
        if (bg == null || !bg.matches("#[0-9a-fA-F]{6}")) return;
        ui(() -> {
                Activity a = act();
                if (a == null) return;
                Window w = a.getWindow();
                int c = Color.parseColor(bg);
                w.setStatusBarColor(c);
                w.setNavigationBarColor(c);
                if (Build.VERSION.SDK_INT >= 30) {
                    WindowInsetsController ic = w.getInsetsController();
                    if (ic != null) {
                        int m = WindowInsetsController.APPEARANCE_LIGHT_STATUS_BARS | WindowInsetsController.APPEARANCE_LIGHT_NAVIGATION_BARS;
                        ic.setSystemBarsAppearance(light ? m : 0, m);
                    }
                } else {
                    View d = w.getDecorView();
                    int f = d.getSystemUiVisibility();
                    int m = View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR | View.SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR;
                    d.setSystemUiVisibility(light ? (f | m) : (f & ~m));
                }
            });
    }

    @JavascriptInterface
    public void moveToBack() {
        ui(() -> {
                Activity a = act();
                if (a != null) a.moveTaskToBack(true);
            });
    }

    @JavascriptInterface
    public void vibrate(int ms) {
        if (ms <= 0) return;
        ms = Math.min(ms, 2000);
        try {
            Vibrator v;
            if (Build.VERSION.SDK_INT >= 31) v = ((VibratorManager) ctx.getSystemService(Context.VIBRATOR_MANAGER_SERVICE)).getDefaultVibrator();
            else v = (Vibrator) ctx.getSystemService(Context.VIBRATOR_SERVICE);
            if (v != null) v.vibrate(VibrationEffect.createOneShot(ms, VibrationEffect.DEFAULT_AMPLITUDE));
        } catch (Exception ignored) {
        }
    }

    /* ---------------- sistema ---------------- */

    private void start(Intent i) {
        try {
            i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            ctx.startActivity(i);
        } catch (Exception ignored) {
        }
    }

    @JavascriptInterface
    public void batterySettings() {
        PowerManager pm = (PowerManager) ctx.getSystemService(Context.POWER_SERVICE);
        if (pm != null && !pm.isIgnoringBatteryOptimizations(ctx.getPackageName()))
            start(new Intent(Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS, Uri.parse("package:" + ctx.getPackageName())));
        else start(new Intent(Settings.ACTION_IGNORE_BATTERY_OPTIMIZATION_SETTINGS));
    }

    @JavascriptInterface
    public void appSettings() {
        start(new Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.parse("package:" + ctx.getPackageName())));
    }

    @JavascriptInterface
    public void share(String text) {
        Intent i = new Intent(Intent.ACTION_SEND).setType("text/plain").putExtra(Intent.EXTRA_TEXT, text);
        start(Intent.createChooser(i, "Compartilhar"));
    }

    @JavascriptInterface
    public void copy(final String text) {
        ui(() -> {
                ClipboardManager cm = (ClipboardManager) ctx.getSystemService(Context.CLIPBOARD_SERVICE);
                ClipData c = ClipData.newPlainText("J-Menseger", text);
                // Chaves (nsec) não aparecem na prévia da área de transferência.
                if (text != null && text.startsWith("nsec1") && Build.VERSION.SDK_INT >= 24) {
                    android.os.PersistableBundle e = new android.os.PersistableBundle();
                    e.putBoolean("android.content.extra.IS_SENSITIVE", true);
                    c.getDescription().setExtras(e);
                }
                cm.setPrimaryClip(c);
            });
    }

    @JavascriptInterface
    public String paste() {
        try {
            ClipboardManager cm = (ClipboardManager) ctx.getSystemService(Context.CLIPBOARD_SERVICE);
            ClipData c = cm.getPrimaryClip();
            if (c == null || c.getItemCount() == 0) return "";
            CharSequence t = c.getItemAt(0).coerceToText(ctx);
            return t == null ? "" : t.toString();
        } catch (Exception e) {
            return "";
        }
    }

    @JavascriptInterface
    public void openUrl(String url) {
        if (url == null) return;
        Uri u = Uri.parse(url);
        String s = u.getScheme();
        if (!"https".equals(s) && !"http".equals(s)) return;
        start(new Intent(Intent.ACTION_VIEW, u));
    }

    /** Música tocando no Spotify (JSON) ou null. */
    @JavascriptInterface
    public String nowPlaying() {
        return App.nowPlaying;
    }

    /** Abre o Spotify (para ligar a transmissão do status do dispositivo). */
    @JavascriptInterface
    public void openSpotify() {
        Intent i = ctx.getPackageManager().getLaunchIntentForPackage("com.spotify.music");
        if (i != null) start(i);
        else start(new Intent(Intent.ACTION_VIEW, Uri.parse("https://play.google.com/store/apps/details?id=com.spotify.music")));
    }
}
