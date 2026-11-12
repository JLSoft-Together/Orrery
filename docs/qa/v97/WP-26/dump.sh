#!/bin/bash
export MSYS_NO_PATHCONV=1
S="adb -s R58N661JMPA"
$S shell uiautomator dump /sdcard/wp26.xml >/dev/null
$S pull /sdcard/wp26.xml ./cur.xml >/dev/null
sed 's/<node/\n<node/g' cur.xml | sed -E 's/.*(text="[^"]*").*(content-desc="[^"]*").*(bounds="[^"]*").*/\1 \2 \3/' | grep -v 'text="" content-desc=""'
