package com.jmenseger.app;

import android.app.Service;
import android.content.Context;
import android.content.Intent;
import android.content.pm.ServiceInfo;
import android.os.Build;
import android.os.IBinder;

/**
 * Mantém o processo (e a página com a conexão aos servidores) vivo com a
 * tela fechada, e marca a chamada como em andamento para o Android não
 * cortar microfone e câmera.
 */
public class ConnService extends Service {
    static volatile boolean running;
    static volatile boolean inCall;
    static volatile boolean callVideo;

    static void sync(Context c) {
        Intent i = new Intent(c, ConnService.class);
        try {
            c.startForegroundService(i);
        } catch (Exception ignored) {
        }
    }

    @Override
    public void onCreate() {
        super.onCreate();
        running = true;
    }

    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
        int tipo = 0;
        if (Build.VERSION.SDK_INT >= 34) {
            tipo = ServiceInfo.FOREGROUND_SERVICE_TYPE_REMOTE_MESSAGING;
            if (inCall) {
                tipo |= ServiceInfo.FOREGROUND_SERVICE_TYPE_MICROPHONE;
                if (callVideo) tipo |= ServiceInfo.FOREGROUND_SERVICE_TYPE_CAMERA;
            }
        } else if (Build.VERSION.SDK_INT >= 30 && inCall) {
            tipo = ServiceInfo.FOREGROUND_SERVICE_TYPE_MICROPHONE | (callVideo ? ServiceInfo.FOREGROUND_SERVICE_TYPE_CAMERA : 0);
        }
        try {
            if (Build.VERSION.SDK_INT >= 29 && tipo != 0) startForeground(Notifier.ID_CONN, Notifier.ongoing(this, inCall, callVideo), tipo);
            else startForeground(Notifier.ID_CONN, Notifier.ongoing(this, inCall, callVideo));
        } catch (Exception e) {
            // Android pode recusar (ex.: câmera sem a tela aberta): segue só com a conexão.
            try {
                if (Build.VERSION.SDK_INT >= 34) startForeground(Notifier.ID_CONN, Notifier.ongoing(this, inCall, false), ServiceInfo.FOREGROUND_SERVICE_TYPE_REMOTE_MESSAGING);
            } catch (Exception ignored) {
                stopSelf();
            }
        }
        // Garante que a página existe para receber mensagens.
        if (!App.get().hasWeb()) App.get().web(getApplicationContext());
        return START_STICKY;
    }

    @Override
    public void onTaskRemoved(Intent rootIntent) {
        super.onTaskRemoved(rootIntent);
    }

    @Override
    public void onDestroy() {
        running = false;
        super.onDestroy();
    }

    @Override
    public IBinder onBind(Intent i) {
        return null;
    }
}
