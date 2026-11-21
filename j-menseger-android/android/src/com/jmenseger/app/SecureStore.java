package com.jmenseger.app;

import android.content.Context;
import android.content.SharedPreferences;
import android.security.keystore.KeyGenParameterSpec;
import android.security.keystore.KeyProperties;
import android.util.Base64;

import java.nio.charset.StandardCharsets;
import java.security.KeyStore;

import javax.crypto.Cipher;
import javax.crypto.KeyGenerator;
import javax.crypto.SecretKey;
import javax.crypto.spec.GCMParameterSpec;

/**
 * Guarda a chave da conta cifrada com AES-GCM por uma chave do Android
 * Keystore (nunca sai do hardware seguro do aparelho, quando ele existe).
 */
final class SecureStore {
    private static final String ALIAS = "jm_master_v1";
    private static final String PREFS = "jm_secure";

    private static SecretKey key() throws Exception {
        KeyStore ks = KeyStore.getInstance("AndroidKeyStore");
        ks.load(null);
        if (!ks.containsAlias(ALIAS)) {
            KeyGenerator kg = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore");
            kg.init(new KeyGenParameterSpec.Builder(ALIAS, KeyProperties.PURPOSE_ENCRYPT | KeyProperties.PURPOSE_DECRYPT)
                    .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                    .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                    .setKeySize(256)
                    .build());
            kg.generateKey();
        }
        return ((KeyStore.SecretKeyEntry) ks.getEntry(ALIAS, null)).getSecretKey();
    }

    private static SharedPreferences prefs(Context c) {
        return c.getSharedPreferences(PREFS, Context.MODE_PRIVATE);
    }

    static synchronized void set(Context c, String k, String v) {
        if (k == null) return;
        if (v == null) {
            del(c, k);
            return;
        }
        String guardado;
        try {
            Cipher ci = Cipher.getInstance("AES/GCM/NoPadding");
            ci.init(Cipher.ENCRYPT_MODE, key());
            byte[] iv = ci.getIV();
            byte[] ct = ci.doFinal(v.getBytes(StandardCharsets.UTF_8));
            byte[] tudo = new byte[iv.length + ct.length];
            System.arraycopy(iv, 0, tudo, 0, iv.length);
            System.arraycopy(ct, 0, tudo, iv.length, ct.length);
            guardado = Base64.encodeToString(tudo, Base64.NO_WRAP);
        } catch (Exception e) {
            // Aparelho com Keystore quebrado: guarda só na área privada do app
            // para não perder a conta.
            guardado = "p:" + v;
        }
        prefs(c).edit().putString(k, guardado).commit();
    }

    static synchronized String get(Context c, String k) {
        String s = prefs(c).getString(k, null);
        if (s == null) return null;
        if (s.startsWith("p:")) return s.substring(2);
        try {
            byte[] tudo = Base64.decode(s, Base64.NO_WRAP);
            Cipher ci = Cipher.getInstance("AES/GCM/NoPadding");
            ci.init(Cipher.DECRYPT_MODE, key(), new GCMParameterSpec(128, tudo, 0, 12));
            return new String(ci.doFinal(tudo, 12, tudo.length - 12), StandardCharsets.UTF_8);
        } catch (Exception e) {
            return null;
        }
    }

    static synchronized void del(Context c, String k) {
        prefs(c).edit().remove(k).commit();
    }
}
