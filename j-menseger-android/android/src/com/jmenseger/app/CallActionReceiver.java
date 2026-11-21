package com.jmenseger.app;

import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;

/** "Recusar" na notificação de chamada, sem abrir o app. */
public class CallActionReceiver extends BroadcastReceiver {
    @Override
    public void onReceive(Context c, Intent i) {
        String a = i.getStringExtra("callAction");
        Notifier.cancelIncoming(c);
        if (a != null && App.get() != null) App.get().runJs("window.JM&&JM.onCallAction(" + App.jsString(a) + ")");
    }
}
