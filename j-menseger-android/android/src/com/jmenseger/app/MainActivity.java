package com.jmenseger.app;

import android.Manifest;
import android.app.Activity;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.view.View;
import android.view.ViewGroup;
import android.view.WindowManager;
import android.webkit.PermissionRequest;
import android.webkit.ValueCallback;
import android.webkit.WebView;
import android.widget.FrameLayout;

import java.util.ArrayList;
import java.util.List;

/** A tela: mostra a WebView do App e faz a ponte com o sistema. */
public class MainActivity extends Activity {
    static MainActivity current;
    /** A tela está aberta na frente agora? */
    static volatile boolean resumed;
    private static final int REQ_FILE = 11;
    private static final int REQ_PERM = 12;
    private static final int REQ_NOTIF = 13;

    private FrameLayout root;
    private ValueCallback<Uri[]> fileCallback;
    private PermissionRequest pendingPerm;

    @Override
    protected void onCreate(Bundle b) {
        super.onCreate(b);
        current = this;
        if (getSharedPreferences(Bridge.PREFS, MODE_PRIVATE).getBoolean("secure", false)) {
            getWindow().addFlags(WindowManager.LayoutParams.FLAG_SECURE);
        }
        root = new FrameLayout(this);
        root.setBackgroundColor(0xFF000000);
        setContentView(root);
        attachWeb();
        handleIntent(getIntent());
    }

    void attachWeb() {
        WebView w = App.get().web(this);
        if (w.getParent() instanceof ViewGroup) ((ViewGroup) w.getParent()).removeView(w);
        root.removeAllViews();
        root.addView(w, new FrameLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT));
    }

    @Override
    protected void onNewIntent(Intent i) {
        super.onNewIntent(i);
        setIntent(i);
        handleIntent(i);
    }

    private void handleIntent(Intent i) {
        if (i == null) return;
        App app = App.get();
        Uri data = i.getData();
        if (Intent.ACTION_VIEW.equals(i.getAction()) && data != null) {
            String s = data.getScheme();
            if ("jmenseger".equals(s) || "nostr".equals(s)) app.runJs("window.JM&&JM.onLink(" + App.jsString(data.toString()) + ")");
        }
        String chat = i.getStringExtra("chat");
        if (chat != null) app.runJs("window.JM&&JM.onOpenChat(" + App.jsString(chat) + ")");
        String acao = i.getStringExtra("callAction");
        if (acao != null) {
            Notifier.cancelIncoming(this);
            app.runJs("window.JM&&JM.onCallAction(" + App.jsString(acao) + ")");
        }
        if (i.getBooleanExtra("callShow", false)) app.runJs("window.JM&&JM.onResume&&JM.onResume()");
    }

    @Override
    protected void onResume() {
        super.onResume();
        current = this;
        resumed = true;
        App.get().rebase(this);
        App.get().runJs("window.JM&&JM.onVisibility(true)");
    }

    @Override
    protected void onPause() {
        super.onPause();
        resumed = false;
        App.get().runJs("window.JM&&JM.onVisibility(false)");
    }

    @Override
    protected void onDestroy() {
        if (current == this) current = null;
        // Sem o serviço de segundo plano, não há por que manter a página viva.
        if (!ConnService.running) App.get().destroyWeb();
        else {
            WebView w = App.get().peekWeb();
            if (w != null && w.getParent() == root) root.removeView(w);
            App.get().rebase(getApplicationContext());
        }
        super.onDestroy();
    }

    @Override
    public void onBackPressed() {
        WebView w = App.get().peekWeb();
        if (w == null) {
            super.onBackPressed();
            return;
        }
        w.evaluateJavascript("(window.JM&&JM.onBack)?JM.onBack():false", (v) -> {
                if (!"true".equals(v)) moveTaskToBack(true);
            });
    }

    /* ---------------- arquivos ---------------- */

    boolean chooseFile(ValueCallback<Uri[]> cb, String[] accept, boolean multiple) {
        if (fileCallback != null) fileCallback.onReceiveValue(null);
        fileCallback = cb;
        String tipo = "*/*";
        if (accept != null && accept.length > 0 && accept[0] != null && accept[0].length() > 0) {
            String a = accept[0].split(",")[0].trim();
            if (a.startsWith(".")) a = "*/*";
            tipo = a;
        }
        Intent i = new Intent(Intent.ACTION_GET_CONTENT);
        i.addCategory(Intent.CATEGORY_OPENABLE);
        i.setType(tipo);
        if (multiple) i.putExtra(Intent.EXTRA_ALLOW_MULTIPLE, true);
        try {
            startActivityForResult(Intent.createChooser(i, tipo.startsWith("audio") ? "Escolher som" : "Escolher foto"), REQ_FILE);
            return true;
        } catch (Exception e) {
            fileCallback = null;
            return false;
        }
    }

    @Override
    protected void onActivityResult(int req, int res, Intent data) {
        if (req == ScreenShare.REQ) {
            ScreenShare.onResult(this, res, data);
            return;
        }
        if (req == REQ_FILE) {
            ValueCallback<Uri[]> cb = fileCallback;
            fileCallback = null;
            if (cb == null) return;
            Uri[] r = null;
            if (res == RESULT_OK && data != null) {
                if (data.getClipData() != null) {
                    int n = data.getClipData().getItemCount();
                    r = new Uri[n];
                    for (int k = 0; k < n; k++) r[k] = data.getClipData().getItemAt(k).getUri();
                } else if (data.getData() != null) {
                    r = new Uri[]{data.getData()};
                }
            }
            cb.onReceiveValue(r);
            return;
        }
        super.onActivityResult(req, res, data);
    }

    /* ---------------- microfone e câmera ---------------- */

    void askMedia(PermissionRequest r) {
        List<String> falta = new ArrayList<String>();
        for (String res : r.getResources()) {
            if (PermissionRequest.RESOURCE_AUDIO_CAPTURE.equals(res) && checkSelfPermission(Manifest.permission.RECORD_AUDIO) != PackageManager.PERMISSION_GRANTED)
                falta.add(Manifest.permission.RECORD_AUDIO);
            if (PermissionRequest.RESOURCE_VIDEO_CAPTURE.equals(res) && checkSelfPermission(Manifest.permission.CAMERA) != PackageManager.PERMISSION_GRANTED)
                falta.add(Manifest.permission.CAMERA);
        }
        if (falta.isEmpty()) {
            grant(r);
            return;
        }
        if (pendingPerm != null) pendingPerm.deny();
        pendingPerm = r;
        if (Build.VERSION.SDK_INT >= 31) falta.add(Manifest.permission.BLUETOOTH_CONNECT);
        requestPermissions(falta.toArray(new String[0]), REQ_PERM);
    }

    private void grant(PermissionRequest r) {
        List<String> ok = new ArrayList<String>();
        for (String res : r.getResources()) {
            if (PermissionRequest.RESOURCE_AUDIO_CAPTURE.equals(res) && checkSelfPermission(Manifest.permission.RECORD_AUDIO) == PackageManager.PERMISSION_GRANTED)
                ok.add(res);
            if (PermissionRequest.RESOURCE_VIDEO_CAPTURE.equals(res) && checkSelfPermission(Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED)
                ok.add(res);
        }
        if (ok.isEmpty()) r.deny();
        else r.grant(ok.toArray(new String[0]));
    }

    void askNotifications() {
        if (Build.VERSION.SDK_INT >= 33) requestPermissions(new String[]{Manifest.permission.POST_NOTIFICATIONS}, REQ_NOTIF);
    }

    @Override
    public void onRequestPermissionsResult(int req, String[] perms, int[] res) {
        if (req == REQ_PERM && pendingPerm != null) {
            PermissionRequest r = pendingPerm;
            pendingPerm = null;
            grant(r);
        }
    }
}
