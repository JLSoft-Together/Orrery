#!/bin/bash
export MSYS_NO_PATHCONV=1
S="adb -s R58N661JMPA"
N=$1
$S shell dumpsys window | grep -q "mCurrentFocus.*dev.ccm.orrery.remote" || { echo "FOREGROUND LOST"; exit 1; }
$S logcat -c
curl -s -X POST http://127.0.0.1:4556/eval --data-binary 'window.__wp26("cls")' >/dev/null
sleep 3
$S logcat -c
curl -s -X POST http://127.0.0.1:4556/eval --data-binary 'window.__wp26("1..3000|%{$_;if($_%25 -eq 0){Start-Sleep -m 15}}")' >/dev/null &
for i in $(seq 1 40); do
  $S shell input swipe 360 500 360 1100 120
  $S shell input swipe 360 1100 360 500 120
done
wait
sleep 6
$S logcat -d -s flutter | grep WP26 | cut -d: -f4- > trial$N.log
cat trial$N.log
