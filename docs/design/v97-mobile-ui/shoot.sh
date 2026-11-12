#!/bin/bash
cd "$(dirname "$0")"
E="/c/Program Files (x86)/Microsoft/Edge/Application/msedge.exe"
U="file:///$(pwd -W)/index.html"
ids=${@:-home-a home-b pair-qr pair-code mon-a mon-b detail ws-a ws-b term term-pick file diff settings lock push empty offline error}
for s in $ids; do for t in light dark; do "$E" --headless=new --disable-gpu --hide-scrollbars --window-size=360,800 --screenshot="$PWD/png/$s-$t.png" "$U?s=$s&t=$t" >/dev/null 2>&1; done; done
