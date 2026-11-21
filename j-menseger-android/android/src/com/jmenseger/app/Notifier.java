package com.jmenseger.app;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.app.Person;
import android.content.Context;
import android.content.Intent;
import android.media.AudioAttributes;
import android.media.RingtoneManager;
import android.os.Build;

import org.json.JSONObject;

/** Notificações: mensagens, chamada chegando e o aviso fixo da conexão. */
final class Notifier {
    static final String CH_MSG = "msgs";
    static final String CH_QUIET = "msgs_quiet";
    static final String CH_CALL = "calls_v1";
    static final String CH_CONN = "conn";
    static final int ID_CALL = 7001;
    static final int ID_CONN = 7000;

    static void channels(Context c) {
        NotificationManager nm = c.getSystemService(NotificationManager.class);
        NotificationChannel m = new NotificationChannel(CH_MSG, "Mensagens", NotificationManager.IMPORTANCE_HIGH);
        m.setDescription("Novas mensagens e convites");
        m.enableVibration(true);
        m.setLockscreenVisibility(Notification.VISIBILITY_PRIVATE);
        NotificationChannel q = new NotificationChannel(CH_QUIET, "Mensagens sem som", NotificationManager.IMPORTANCE_DEFAULT);
        q.setSound(null, null);
        q.enableVibration(false);
        NotificationChannel ch = new NotificationChannel(CH_CALL, "Chamadas recebidas", NotificationManager.IMPORTANCE_HIGH);
        ch.setSound(RingtoneManager.getDefaultUri(RingtoneManager.TYPE_RINGTONE),
                new AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_NOTIFICATION_RINGTONE).setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION).build());
        ch.enableVibration(true);
        ch.setVibrationPattern(new long[]{0, 700, 500, 700, 500, 700});
        ch.setLockscreenVisibility(Notification.VISIBILITY_PUBLIC);
        NotificationChannel conn = new NotificationChannel(CH_CONN, "Conexão em segundo plano", NotificationManager.IMPORTANCE_MIN);
        conn.setDescription("Aviso fixo enquanto o app recebe mensagens com a tela fechada");
        conn.setShowBadge(false);
        nm.createNotificationChannel(m);
        nm.createNotificationChannel(q);
        nm.createNotificationChannel(ch);
        nm.createNotificationChannel(conn);
    }

    static PendingIntent open(Context c, int req, String extra, String value) {
        Intent i = new Intent(c, MainActivity.class).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_SINGLE_TOP);
        if (extra != null) i.putExtra(extra, value);
        return PendingIntent.getActivity(c, req, i, PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);
    }

    static void message(Context c, String json) {
        try {
            JSONObject o = new JSONObject(json);
            String tag = o.optString("tag", "msg");
            boolean silent = o.optBoolean("silent", false);
            Notification.Builder b = new Notification.Builder(c, silent ? CH_QUIET : CH_MSG)
                    .setSmallIcon(R.drawable.ic_stat)
                    .setColor(0xFF2FD47C)
                    .setContentTitle(o.optString("title", "J-Menseger"))
                    .setContentText(o.optString("body", "Nova mensagem"))
                    .setStyle(new Notification.BigTextStyle().bigText(o.optString("body", "")))
                    .setAutoCancel(true)
                    .setCategory(Notification.CATEGORY_MESSAGE)
                    .setVisibility(Notification.VISIBILITY_PRIVATE)
                    .setPublicVersion(new Notification.Builder(c, CH_MSG).setSmallIcon(R.drawable.ic_stat).setContentTitle("J-Menseger").setContentText("Nova mensagem").build())
                    .setContentIntent(open(c, tag.hashCode(), "chat", o.optString("chat", null)));
            c.getSystemService(NotificationManager.class).notify(tag, 1, b.build());
        } catch (Exception ignored) {
        }
    }

    static void cancel(Context c, String tag) {
        if (tag == null) return;
        c.getSystemService(NotificationManager.class).cancel(tag, 1);
    }

    /** Chamada chegando: toca, vibra e abre em tela cheia com o celular bloqueado. */
    static void incomingCall(Context c, String json) {
        try {
            JSONObject o = new JSONObject(json);
            String nome = o.optString("name", "Alguém");
            boolean video = o.optBoolean("video", false);
            PendingIntent tela = open(c, 31, "callShow", "1");
            Intent ia = new Intent(c, CallActionReceiver.class).putExtra("callAction", "accept");
            Intent ir = new Intent(c, CallActionReceiver.class).putExtra("callAction", "decline");
            PendingIntent aceitar = PendingIntent.getActivity(c, 32,
                    new Intent(c, MainActivity.class).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_SINGLE_TOP).putExtra("callAction", "accept"),
                    PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);
            PendingIntent recusar = PendingIntent.getBroadcast(c, 33, ir, PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);
            Notification.Builder b = new Notification.Builder(c, CH_CALL)
                    .setSmallIcon(R.drawable.ic_stat)
                    .setColor(0xFF2FD47C)
                    .setContentTitle(nome)
                    .setContentText(video ? "Chamada de vídeo no J-Menseger" : "Chamada de voz no J-Menseger")
                    .setCategory(Notification.CATEGORY_CALL)
                    .setOngoing(true)
                    .setVisibility(Notification.VISIBILITY_PUBLIC)
                    .setContentIntent(tela)
                    .setFullScreenIntent(tela, true)
                    .setTimeoutAfter(45000);
            if (Build.VERSION.SDK_INT >= 31) {
                Person p = new Person.Builder().setName(nome).setImportant(true).build();
                b.setStyle(Notification.CallStyle.forIncomingCall(p, recusar, aceitar).setIsVideo(video));
            } else {
                b.addAction(new Notification.Action.Builder(null, "Recusar", recusar).build());
                b.addAction(new Notification.Action.Builder(null, "Atender", aceitar).build());
            }
            c.getSystemService(NotificationManager.class).notify(ID_CALL, b.build());
        } catch (Exception ignored) {
        }
    }

    static void cancelIncoming(Context c) {
        c.getSystemService(NotificationManager.class).cancel(ID_CALL);
    }

    /** Aviso fixo do serviço de segundo plano. */
    static Notification ongoing(Context c, boolean call, boolean video) {
        Notification.Builder b = new Notification.Builder(c, call ? CH_QUIET : CH_CONN)
                .setSmallIcon(R.drawable.ic_stat)
                .setColor(0xFF2FD47C)
                .setOngoing(true)
                .setShowWhen(false)
                .setContentIntent(open(c, 30, null, null));
        if (call) {
            b.setContentTitle(video ? "Chamada de vídeo em andamento" : "Chamada em andamento")
                    .setContentText("Toque para voltar à chamada")
                    .setCategory(Notification.CATEGORY_CALL)
                    .setOnlyAlertOnce(true);
        } else {
            b.setContentTitle("J-Menseger conectado").setContentText("Recebendo mensagens cifradas em segundo plano").setCategory(Notification.CATEGORY_SERVICE);
        }
        return b.build();
    }
}
