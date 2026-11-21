package com.jmenseger.app;

import android.net.Uri;
import android.util.Log;
import android.webkit.ConsoleMessage;
import android.webkit.PermissionRequest;
import android.webkit.ValueCallback;
import android.webkit.WebChromeClient;
import android.webkit.WebView;

/** Microfone, câmera e escolha de arquivos para a página. */
public class ChromeClient extends WebChromeClient {

    static String originOf(Uri u) {
        return u == null ? "" : u.getScheme() + "://" + u.getHost();
    }

    @Override
    public void onPermissionRequest(final PermissionRequest r) {
        // Só a página do próprio app pode pedir microfone e câmera.
        if (!App.ORIGIN.equals(originOf(r.getOrigin()))) {
            r.deny();
            return;
        }
        MainActivity a = MainActivity.current;
        if (a == null) {
            r.deny();
            return;
        }
        a.askMedia(r);
    }

    @Override
    public boolean onShowFileChooser(WebView v, ValueCallback<Uri[]> cb, FileChooserParams p) {
        MainActivity a = MainActivity.current;
        if (a == null) return false;
        return a.chooseFile(cb, p.getAcceptTypes(), p.getMode() == FileChooserParams.MODE_OPEN_MULTIPLE);
    }

    @Override
    public boolean onConsoleMessage(ConsoleMessage m) {
        if (BuildInfo.DEBUG) Log.d("jm", m.message());
        return true;
    }
}
