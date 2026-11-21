package com.jmenseger.app;

import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;

/** Liga a conexão em segundo plano depois de reiniciar o celular (se estava ligada). */
public class BootReceiver extends BroadcastReceiver {
    @Override
    public void onReceive(Context c, Intent i) {
        String a = i.getAction();
        if (!Intent.ACTION_BOOT_COMPLETED.equals(a) && !Intent.ACTION_MY_PACKAGE_REPLACED.equals(a)) return;
        if (c.getSharedPreferences(Bridge.PREFS, Context.MODE_PRIVATE).getBoolean("bg", false)) ConnService.sync(c);
    }
}
