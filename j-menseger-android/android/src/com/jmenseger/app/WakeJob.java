package com.jmenseger.app;

import android.app.job.JobInfo;
import android.app.job.JobParameters;
import android.app.job.JobScheduler;
import android.app.job.JobService;
import android.content.ComponentName;
import android.content.Context;
import android.os.Handler;
import android.os.Looper;

/**
 * Rede de segurança das notificações com o app fechado.
 *
 * O serviço em segundo plano mantém a conexão aberta; mas o Android (e
 * principalmente alguns fabricantes) às vezes mata o processo mesmo assim. A
 * cada ~15 minutos, com internet, este trabalho confere: se a conexão caiu,
 * religa o serviço; se o Android não deixar, abre a página sem tela por meio
 * minuto, que basta para buscar o que chegou e mostrar as notificações.
 */
public class WakeJob extends JobService {
    static final int ID = 7701;
    private final Handler main = new Handler(Looper.getMainLooper());

    static void schedule(Context c) {
        try {
            JobScheduler js = (JobScheduler) c.getSystemService(Context.JOB_SCHEDULER_SERVICE);
            if (js == null) return;
            for (JobInfo j : js.getAllPendingJobs()) if (j.getId() == ID) return;
            JobInfo info = new JobInfo.Builder(ID, new ComponentName(c, WakeJob.class))
                    .setRequiredNetworkType(JobInfo.NETWORK_TYPE_ANY)
                    .setPeriodic(15 * 60 * 1000L)
                    .setPersisted(true)
                    .build();
            js.schedule(info);
        } catch (Exception ignored) {
        }
    }

    static void cancel(Context c) {
        try {
            JobScheduler js = (JobScheduler) c.getSystemService(Context.JOB_SCHEDULER_SERVICE);
            if (js != null) js.cancel(ID);
        } catch (Exception ignored) {
        }
    }

    @Override
    public boolean onStartJob(final JobParameters params) {
        boolean ligado = getSharedPreferences(Bridge.PREFS, MODE_PRIVATE).getBoolean("bg", false);
        if (!ligado || ConnService.running || MainActivity.resumed) return false;
        ConnService.sync(this);
        main.postDelayed(() -> {
            // O Android não deixou ligar o serviço: abre a página sem tela.
            if (!ConnService.running && App.get() != null && !App.get().hasWeb()) App.get().web(getApplicationContext());
        }, 1500);
        main.postDelayed(() -> jobFinished(params, false), 30000);
        return true;
    }

    @Override
    public boolean onStopJob(JobParameters params) {
        return false;
    }
}
