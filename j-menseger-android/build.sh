#!/bin/bash
# Monta o APK do J-Menseger com as ferramentas oficiais do Android, sem Gradle.
#
#   ANDROID_BT   pasta do build-tools (aapt2, d8, zipalign, apksigner)
#   ANDROID_JAR  android.jar da plataforma 34
#   KEYSTORE     chave de assinatura (padrão: keystore/jmenseger.jks, com a senha em keystore/senha.txt)
set -euo pipefail
cd "$(dirname "$0")"
VERSAO="${VERSAO:-0.7.0-beta}"
CODIGO="${CODIGO:-7}"
BT="${ANDROID_BT:-/opt/android/android-14}"
AJ="${ANDROID_JAR:-/opt/android/android-34/android.jar}"
KS="${KEYSTORE:-keystore/jmenseger.jks}"
OUT=build/out
rm -rf "$OUT" && mkdir -p "$OUT/assets/www" "$OUT/classes" "$OUT/gen"

echo "▸ Página"
[ -d node_modules/esbuild ] || npm install --silent
W="$OUT/assets/www"
cp -r web/fonts web/sounds web/index.html web/jsqr.js "$W/"
npx esbuild web/app.js --minify --target=chrome100 --legal-comments=eof --outfile="$W/app.js" --log-level=warning
npx esbuild web/styles.css --minify --outfile="$W/styles.css" --log-level=warning

echo "▸ Recursos"
"$BT/aapt2" compile --dir android/res -o "$OUT/res.zip"
"$BT/aapt2" link -o "$OUT/base.apk" -I "$AJ" --manifest android/AndroidManifest.xml --java "$OUT/gen" \
  --min-sdk-version 26 --target-sdk-version 34 --version-code "$CODIGO" --version-name "$VERSAO" \
  -A "$OUT/assets" -0 woff2 -0 ogg "$OUT/res.zip"

echo "▸ Java"
javac -nowarn -Xlint:-options -g:source,lines -source 8 -target 8 -encoding UTF-8 -bootclasspath "$AJ:$BT/core-lambda-stubs.jar" -classpath "$AJ" -d "$OUT/classes" \
  $(find android/src "$OUT/gen" -name '*.java')
"$BT/d8" --release --min-api 26 --lib "$AJ" --output "$OUT" $(find "$OUT/classes" -name '*.class')

echo "▸ Empacotar e assinar"
cp "$OUT/base.apk" "$OUT/unsigned.apk"
(cd "$OUT" && zip -q -j unsigned.apk classes.dex)
"$BT/zipalign" -f -p 4 "$OUT/unsigned.apk" "$OUT/aligned.apk"
if [ ! -f "$KS" ]; then
  mkdir -p "$(dirname "$KS")"
  head -c 24 /dev/urandom | base64 | tr -d '/+=' > "$(dirname "$KS")/senha.txt"
  keytool -genkeypair -keystore "$KS" -storetype PKCS12 -alias jmenseger -keyalg RSA -keysize 3072 -validity 10000 \
    -storepass "$(cat "$(dirname "$KS")/senha.txt")" -dname "CN=J-Menseger, O=J-Menseger, C=BR" 2>/dev/null
  echo "nova chave criada em $(dirname "$KS")/ (guarde-a: sem ela não dá para atualizar o app instalado)"
fi
APK="build/J-Menseger-$VERSAO.apk"
"$BT/apksigner" sign --ks "$KS" --ks-pass "pass:$(cat "$(dirname "$KS")/senha.txt")" --out "$APK" "$OUT/aligned.apk"
"$BT/apksigner" verify "$APK"
ls -la "$APK"
