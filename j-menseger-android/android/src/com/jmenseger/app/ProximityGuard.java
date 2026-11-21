package com.jmenseger.app;

import android.content.Context;
import android.hardware.Sensor;
import android.hardware.SensorEvent;
import android.hardware.SensorEventListener;
import android.hardware.SensorManager;
import android.os.Handler;
import android.os.Looper;
import android.os.PowerManager;

/**
 * Apaga a tela quando o celular está na orelha, numa chamada de voz.
 *
 * O jeito comum (segurar a trava de proximidade a chamada inteira) apaga a
 * tela no primeiro instante em que o sensor diz "perto" — um dedo, a mão ou
 * um reflexo perto do sensor já bastam, e a tela pisca preta. Aqui a trava
 * só é pega depois de o sensor ficar "perto" por um tempo contínuo, e é
 * solta assim que ele diz "longe".
 */
final class ProximityGuard implements SensorEventListener {
    private static final long ESPERA_MS = 650;
    private final Context ctx;
    private final Handler main = new Handler(Looper.getMainLooper());
    private SensorManager sm;
    private Sensor sensor;
    private PowerManager.WakeLock lock;
    private boolean ligado;
    private boolean perto;
    private final Runnable apaga = () -> {
        if (ligado && perto) acquire();
    };

    ProximityGuard(Context ctx) {
        this.ctx = ctx.getApplicationContext();
    }

    void start() {
        if (ligado) return;
        try {
            sm = (SensorManager) ctx.getSystemService(Context.SENSOR_SERVICE);
            sensor = sm == null ? null : sm.getDefaultSensor(Sensor.TYPE_PROXIMITY);
            PowerManager pm = (PowerManager) ctx.getSystemService(Context.POWER_SERVICE);
            if (sensor == null || pm == null || !pm.isWakeLockLevelSupported(PowerManager.PROXIMITY_SCREEN_OFF_WAKE_LOCK)) return;
            if (lock == null) lock = pm.newWakeLock(PowerManager.PROXIMITY_SCREEN_OFF_WAKE_LOCK, "jmenseger:proximity");
            ligado = true;
            perto = false;
            sm.registerListener(this, sensor, SensorManager.SENSOR_DELAY_NORMAL, main);
        } catch (Exception ignored) {
        }
    }

    void stop() {
        ligado = false;
        perto = false;
        main.removeCallbacks(apaga);
        try {
            if (sm != null) sm.unregisterListener(this);
        } catch (Exception ignored) {
        }
        release();
    }

    @Override
    public void onSensorChanged(SensorEvent e) {
        if (!ligado || sensor == null) return;
        float v = e.values.length > 0 ? e.values[0] : sensor.getMaximumRange();
        boolean agoraPerto = v < Math.min(3f, sensor.getMaximumRange());
        if (agoraPerto == perto) return;
        perto = agoraPerto;
        main.removeCallbacks(apaga);
        if (perto) main.postDelayed(apaga, ESPERA_MS);
        else release();
    }

    @Override
    public void onAccuracyChanged(Sensor s, int a) {
    }

    private void acquire() {
        try {
            if (lock != null && !lock.isHeld()) lock.acquire(4 * 60 * 60 * 1000L);
        } catch (Exception ignored) {
        }
    }

    private void release() {
        try {
            if (lock != null && lock.isHeld()) lock.release();
        } catch (Exception ignored) {
        }
    }
}
