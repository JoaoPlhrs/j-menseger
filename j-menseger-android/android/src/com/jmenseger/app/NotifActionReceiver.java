package com.jmenseger.app;

import android.app.RemoteInput;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.os.Bundle;

/** "Responder" e "Marcar como lida" na notificação de mensagem, sem abrir o app. */
public class NotifActionReceiver extends BroadcastReceiver {
    @Override
    public void onReceive(Context c, Intent i) {
        String chat = i.getStringExtra("chat");
        String acao = i.getStringExtra("acao");
        if (chat == null || acao == null) return;
        App app = App.get();
        if (app == null) return;
        // Com o app fechado a página não existe: cria sem tela (o motor sobe e manda).
        if (!app.hasWeb()) app.web(c.getApplicationContext());
        if ("reply".equals(acao)) {
            Bundle r = RemoteInput.getResultsFromIntent(i);
            CharSequence t = r == null ? null : r.getCharSequence("r");
            String texto = t == null ? "" : t.toString();
            app.runJs("window.JM&&JM.onNotifReply&&JM.onNotifReply(" + App.jsString(chat) + "," + App.jsString(texto) + ")");
        } else {
            app.runJs("window.JM&&JM.onNotifRead&&JM.onNotifRead(" + App.jsString(chat) + ")");
        }
        // Tira a notificação (senão o Android deixa o "enviando…" girando nela).
        Notifier.cancel(c, chat);
    }
}
