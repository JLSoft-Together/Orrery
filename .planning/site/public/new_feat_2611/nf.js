(() => {
  const $ = (s, r = document) => r.querySelector(s)
  const $$ = (s, r = document) => [...r.querySelectorAll(s)]
  const RM = matchMedia('(prefers-reduced-motion: reduce)')

  const P = {
    check: '<path d="M20 6 9 17l-5-5"/>',
    x: '<path d="M18 6 6 18M6 6l12 12"/>',
    spin: '<path d="M21 12a9 9 0 1 1-6.2-8.6"/>',
    phone: '<rect x="7" y="2" width="10" height="20" rx="2"/><path d="M11 18h2"/>',
    usb: '<path d="M12 2v5M9 4h6M8 7h8v6a4 4 0 0 1-8 0zM12 17v5"/>',
    qr: '<rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><path d="M14 14h3v3M21 14v.01M14 21h7v-4"/>',
    dl: '<path d="M12 3v12M7 10l5 5 5-5M5 21h14"/>',
    wifi: '<path d="M5 12.5a10 10 0 0 1 14 0M8.5 16a5 5 0 0 1 7 0M2 9a15 15 0 0 1 20 0M12 20h.01"/>',
    cloud: '<path d="M17.5 19H9a7 7 0 1 1 6.7-9h1.8a4.5 4.5 0 1 1 0 9z"/>',
    bell: '<path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9M10.3 21a2 2 0 0 0 3.4 0"/>',
    alert: '<circle cx="12" cy="12" r="10"/><path d="M12 8v4M12 16h.01"/>',
    shield: '<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><path d="m9 12 2 2 4-4"/>',
    copy: '<rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>',
    play: '<path d="M7 4l13 8-13 8z"/>',
    pause: '<path d="M7 4h4v16H7zM14 4h4v16h-4z"/>',
    reset: '<path d="M3 12a9 9 0 1 0 3-6.7L3 8M3 3v5h5"/>',
    chev: '<path d="m9 18 6-6-6-6"/>',
    back: '<path d="m15 18-6-6 6-6"/>',
    moon: '<path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9z"/>',
    sun: '<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/>',
    monitor: '<rect x="2" y="3" width="20" height="14" rx="2"/><path d="M8 21h8M12 17v4"/>',
    mic: '<rect x="9" y="2" width="6" height="12" rx="3"/><path d="M19 10v1a7 7 0 0 1-14 0v-1M12 18v4"/>',
    cam: '<path d="M14.5 4h-5L7 7H4a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-3z"/><circle cx="12" cy="13" r="3"/>',
    plus: '<path d="M12 5v14M5 12h14"/>',
    undo: '<path d="M9 14 4 9l5-5M4 9h11a5 5 0 0 1 0 10h-1"/>',
    globe: '<circle cx="12" cy="12" r="10"/><path d="M2 12h20M12 2a15 15 0 0 1 0 20M12 2a15 15 0 0 0 0 20"/>',
    term: '<path d="m4 17 6-6-6-6M12 19h8"/>',
    bot: '<rect x="4" y="8" width="16" height="12" rx="2"/><path d="M12 4v4M9 14h.01M15 14h.01"/>',
    send: '<path d="M22 2 11 13M22 2l-7 20-4-9-9-4z"/>',
    link: '<path d="M10 13a5 5 0 0 0 7.5.5l3-3a5 5 0 0 0-7-7l-1.8 1.7M14 11a5 5 0 0 0-7.5-.5l-3 3a5 5 0 0 0 7 7l1.7-1.7"/>',
    clock: '<circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/>',
    lock: '<rect x="4" y="11" width="16" height="10" rx="2"/><path d="M8 11V7a4 4 0 0 1 8 0v4"/>',
    gear: '<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-2.9 1.2V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-2.9-1.2l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1A1.7 1.7 0 0 0 3 15.4H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.2-2.9l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1A1.7 1.7 0 0 0 9 3.6V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 2.9 1.2l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0 1.2 2.9h.1a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z"/>',
    pencil: '<path d="M17 3a2.8 2.8 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5z"/>',
    zap: '<path d="M13 2 3 14h9l-1 8 10-12h-9z"/>',
    home: '<path d="M3 10l9-7 9 7v10a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>',
    rotate: '<path d="M21 12a9 9 0 1 1-3-6.7L21 8M21 3v5h-5"/>',
    trash: '<path d="M3 6h18M8 6V4h8v2M19 6l-1 14H6L5 6"/>',
  }
  const I = (n, s = 16, cls = '') => `<svg class="${n === 'spin' ? 'spin ' : ''}${cls}" width="${s}" height="${s}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${P[n]}</svg>`
  const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c])

  function qr(seed, dim) {
    let s = seed >>> 0
    const rnd = () => ((s = (s * 1664525 + 1013904223) >>> 0) / 4294967296)
    const n = 25
    const on = (x, y) => {
      const f = (fx, fy) => x >= fx && x < fx + 7 && y >= fy && y < fy + 7
      for (const [fx, fy] of [[0, 0], [18, 0], [0, 18]]) {
        if (f(fx, fy)) {
          const dx = x - fx, dy = y - fy
          return dx === 0 || dx === 6 || dy === 0 || dy === 6 || (dx >= 2 && dx <= 4 && dy >= 2 && dy <= 4)
        }
        if (x >= fx - 1 && x <= fx + 7 && y >= fy - 1 && y <= fy + 7) return false
      }
      return rnd() > 0.52
    }
    let r = ''
    for (let y = 0; y < n; y++) for (let x = 0; x < n; x++) if (on(x, y)) r += `<rect x="${x}" y="${y}" width="1.02" height="1.02"/>`
    return `<div class="qr${dim ? ' dim' : ''}" role="img" aria-label="Mã QR minh hoạ"><svg viewBox="0 0 25 25" fill="#17171c" shape-rendering="crispEdges">${r}</svg></div>`
  }

  const cdEnds = new Map()
  function countdown(key, secs) {
    if (!cdEnds.has(key)) cdEnds.set(key, Date.now() + secs * 1000)
    return `<span class="cd" data-cd="${key}" data-total="${secs}"><svg class="ring" viewBox="0 0 20 20"><circle class="bg" cx="10" cy="10" r="8"/><circle class="fg" cx="10" cy="10" r="8" stroke-dasharray="50.3" stroke-dashoffset="0"/></svg><span class="count">--:--</span></span>`
  }
  function tickCd() {
    $$('[data-cd]').forEach((el) => {
      const end = cdEnds.get(el.dataset.cd)
      if (!end) return
      const left = Math.max(0, Math.round((end - Date.now()) / 1000))
      const total = +el.dataset.total
      $('.count', el).textContent = `${Math.floor(left / 60)}:${String(left % 60).padStart(2, '0')}`
      $('.fg', el).setAttribute('stroke-dashoffset', String(50.3 * (1 - left / total)))
    })
  }
  setInterval(tickCd, 1000)

  const SIDE = ['Chung', 'Giao diện', 'Thông báo', 'Điện thoại, thiết bị', 'Telegram', 'Phím tắt', 'Sao lưu']
  const win = (title, body, side = 3) => `
    <div class="win-bar"><i></i><i></i><i></i><span>${esc(title)}</span></div>
    <div class="win-body">
      <div class="win-side" aria-hidden="true">${SIDE.map((s, i) => `<a class="${i === side ? 'on' : ''}">${s}</a>`).join('')}</div>
      <div class="win-main">${body}</div>
    </div>`

  const sb = () => '<div class="sb"><span>9:41</span><span>5G 82%</span></div>'
  const appsGrid = (withApp) => `<div class="apps">${['Camera', 'Ảnh', 'Chrome', 'Zalo', 'Cài đặt', 'Gmail', 'Drive', withApp ? 'Orrery' : 'Lịch'].map((n) => `<span class="${n === 'Orrery' ? 'orr' : ''}"><i>${n === 'Orrery' ? I('globe', 20) : ''}</i>${n}</span>`).join('')}</div>`
  const orrIcon = `<i>${I('globe', 11)}</i>`

  const STEP_NAMES = ['Cài app', 'Ghép đôi', 'Kiểm tra']
  function stepper(cur) {
    let h = '<div class="steps" aria-label="Tiến trình">'
    STEP_NAMES.forEach((n, i) => {
      const k = i + 1
      const cls = k < cur ? 'done' : k === cur ? 'on' : ''
      h += `<div class="s ${cls}" ${k === cur ? 'aria-current="step"' : ''}><b>${k < cur ? I('check', 12) : k}</b><span>${n}</span></div>`
      if (k < 3) h += `<div class="ln ${k < cur ? 'done' : ''}"></div>`
    })
    return h + '</div>'
  }
  const wizHead = (cur, back) => `<div style="display:flex;align-items:center;gap:8px;flex-wrap:wrap"><button class="b link" data-go="${back}" aria-label="Quay lại">${I('back', 14)} Quay lại</button><p class="m-h" style="font-size:16px;flex:1 1 auto">Kết nối điện thoại</p></div>${stepper(cur)}`

  const ADV = `
    <div class="adv" data-adv>
      <button type="button" data-toggle-adv aria-expanded="false">${I('gear', 15)}<span class="grow">Nâng cao</span><span class="tag">Relay, push, bảo mật, nhật ký</span>${I('chev', 15, 'chev')}</button>
      <div class="adv-b"><div>
        <div class="row">${'<span class="ic">' + I('cloud', 15) + '</span>'}<div class="grow"><p class="t">Relay Cloudflare</p><p class="d">Dùng khi điện thoại ở ngoài Wi-Fi nhà</p></div><span class="tag ok">${I('check', 11)} Online</span></div>
        <div class="row"><span class="ic">${I('bell', 15)}</span><div class="grow"><p class="t">Push FCM / Web Push</p><p class="d">Thông báo khi app điện thoại đang tắt</p></div><span class="tag ok">${I('check', 11)} Đã cài</span></div>
        <div class="row"><span class="ic">${I('zap', 15)}</span><div class="grow"><p class="t">Giữ máy thức khi có điện thoại kết nối</p></div><button class="sw" role="switch" aria-checked="true" aria-label="Giữ máy thức" data-sw></button></div>
        <div class="row"><span class="ic">${I('shield', 15)}</span><div class="grow"><p class="t">Bảo mật</p><p class="d">Máy bị chặn (0), địa chỉ tin cậy (2), nhật ký truy cập</p></div>${I('chev', 15)}</div>
        <div class="row"><span class="ic">${I('wifi', 15)}</span><div class="grow"><p class="t">Cổng 4320, firewall</p><p class="d">Windows Firewall đã cho phép mạng Private</p></div>${I('chev', 15)}</div>
      </div></div>
    </div>`

  const usbTile = (sc) => sc === 'usb'
    ? `<div class="tile best"><span class="ic acc">${I('usb', 16)}</span><div class="grow"><span class="ribbon">Nhanh nhất</span><p class="t">Cài qua cáp USB</p><p class="d">Galaxy A33 đang cắm cáp, adb sẵn sàng</p></div><button class="b pri" data-go="usbInstalling">${I('dl', 14)} Cài lên Galaxy A33</button></div>`
    : `<div class="tile"><span class="ic">${I('usb', 16)}</span><div class="grow"><p class="t">Cài qua cáp USB</p><p class="d">Chưa thấy máy Android nào cắm cáp. Cắm cáp và bật Gỡ lỗi USB, nút cài sẽ tự hiện.</p></div><button class="b" disabled>Chờ cắm cáp</button></div>`
  const qrTile = (sc) => `<div class="tile ${sc === 'qr' ? 'best' : ''}"><span class="ic ${sc === 'qr' ? 'acc' : ''}">${I('qr', 16)}</span><div class="grow">${sc === 'qr' ? '<span class="ribbon">Gợi ý</span>' : ''}<p class="t">Quét QR để tải</p><p class="d">Điện thoại dùng chung Wi-Fi với máy này. 1 mã cho cả tải lẫn ghép đôi.</p></div><button class="b ${sc === 'qr' ? 'pri' : ''}" data-go="qrInstall">${I('qr', 14)} Hiện QR</button></div>`
  const fileTile = `<div class="tile"><span class="ic">${I('dl', 16)}</span><div class="grow"><p class="t">Tải file APK</p><p class="d">Orrery Remote 4.0.10, 24.1 MB (minh hoạ). Gửi qua Zalo, Drive rồi cài.</p></div><div style="display:flex;gap:6px;flex-wrap:wrap"><button class="b" data-toast="Đã lưu orrery-remote-4.0.10.apk vào Downloads">Lưu vào Downloads</button><button class="b" data-toast="Đã copy link tải">${I('copy', 13)} Copy link</button></div></div>`
  const platSeg = (ios) => `<div class="seg-m" role="tablist" aria-label="Loại điện thoại"><button role="tab" aria-selected="${!ios}" class="${ios ? '' : 'on'}" data-go="install">Android</button><button role="tab" aria-selected="${ios}" class="${ios ? 'on' : ''}" data-go="installIos">iPhone, iPad</button></div>`

  const phHome = (withApp, extra = '') => `${sb()}<div class="pb" style="background:var(--m-raised)">${extra}${appsGrid(withApp)}</div>`
  const phApp = (body) => `${sb()}<div class="pb"><div style="display:flex;align-items:center;gap:8px"><span class="ic acc" style="width:26px;height:26px">${I('globe', 14)}</span><b>Orrery Remote</b></div>${body}</div>`
  const sessionsList = `<div class="ph-card"><p class="t">DESKTOP-JIN</p><p class="d">${I('check', 11)} Đã kết nối qua Wi-Fi</p></div><div class="ph-card"><p class="t">api-server, Claude</p><p class="d">Đang chạy, Edit auth.ts</p></div><div class="ph-card"><p class="t">web-app, Codex</p><p class="d">Chờ bạn trả lời</p></div>`

  const CN = {
    off: {
      label: 'off', step: 0,
      desk: () => `<div><p class="m-h">Điện thoại và thiết bị</p><p class="m-sub">Điều khiển Orrery từ điện thoại; cài app lên máy Android đang cắm.</p></div>
        <div class="panel hero-c"><img src="img/ill04.webp" alt="" width="120" height="120"><div><p class="t" style="font-weight:700;font-size:15px;margin:0">Chưa có điện thoại nào</p><p class="d" style="font-size:13px;color:var(--m-muted);margin:4px 0 10px">Kết nối 1 lần để nhận thông báo, trả lời agent, mở pane từ điện thoại.</p><button class="b pri lg" data-go="consent">${I('phone', 15)} Kết nối điện thoại</button></div></div>
        <div class="panel"><div class="row"><span class="ic">${I('phone', 15)}</span><div class="grow"><p class="t">Galaxy A33</p><div class="tags"><span class="tag">${I('usb', 11)} USB adb</span><span class="tag wait">Chưa cài Remote</span></div></div><button class="b" data-go="consent">Cài Remote</button></div>
        <div class="row"><span class="ic">${I('bot', 15)}</span><div class="grow"><p class="t">Medium_Phone_API_35</p><div class="tags"><span class="tag">Emulator</span><span class="tag">Đang tắt</span></div></div></div></div>${ADV}`,
      phone: () => phHome(false),
      cap: 'Mở Settings: 1 nút chính, thiết bị adb đang cắm hiện sẵn kèm "Cài Remote". Relay, push, bảo mật gập vào "Nâng cao" (bấm để mở).',
      next: 'Bấm "Kết nối điện thoại"',
    },
    consent: {
      label: 'consent', step: 0, base: 'off',
      sheet: () => `<div class="sheet" role="dialog" aria-modal="true" aria-labelledby="cs-h"><h4 id="cs-h">Cho phép điện thoại điều khiển máy này?</h4><p>Điện thoại đã ghép đôi sẽ làm được:</p><ul><li>${I('term', 15)}<span>Mở, đóng pane; gõ và gửi prompt</span></li><li>${I('pencil', 15)}<span>Sửa file, chạy git và lệnh</span></li><li>${I('shield', 15)}<span>Env, secret, app key không rời máy. Tắt lúc nào cũng được.</span></li></ul><div class="acts"><button class="b" data-go="off">Huỷ</button><button class="b pri" data-go="${'@install'}">Bật và tiếp tục</button></div></div>`,
      phone: () => phHome(false),
      cap: 'Công tắc "Bật điều khiển từ xa" gộp vào nút chính, nhưng vẫn hỏi rõ 1 lần vì remote control tắt mặc định (core-principle, ngoại lệ 7).',
      next: '"Bật và tiếp tục" hoặc Huỷ',
    },
    install: {
      label: 'install', step: 1,
      desk: (sc) => `${wizHead(1, 'off')}${platSeg(false)}<div class="tiles">${sc === 'qr' ? qrTile(sc) + usbTile(sc) : usbTile(sc) + qrTile(sc)}${fileTile}</div><button class="b link" data-go="pairWait" style="align-self:flex-start">Đã cài app rồi? Sang ghép đôi ${I('chev', 13)}</button>`,
      phone: () => phHome(false),
      cap: 'Bước 1 tự chọn Android vì có máy adb. Có cáp thì "Cài qua cáp USB" đứng đầu; không có cáp thì QR đứng đầu, ô USB nói rõ cần làm gì. APK có sẵn, không còn "Chọn APK…".',
      next: 'Chọn 1 trong 3 cách',
    },
    usbInstalling: {
      label: 'usbInstalling', step: 1, auto: ['pairConfirm', 2600],
      desk: () => `${wizHead(1, 'install')}<div class="panel"><ul class="chk"><li><span class="ic ok">${I('check', 14)}</span><span class="grow">APK Orrery Remote 4.0.10 có sẵn trong bản cài</span></li><li><span class="ic acc">${I('spin', 14)}</span><span class="grow">Cài lên Galaxy A33<div class="prog"><i style="--p:0" data-grow></i></div></span><span class="v" data-pct>0%</span></li><li><span class="ic">${I('link', 14)}</span><span class="grow" style="color:var(--m-muted)">Mở app, gửi mã ghép đôi qua cáp</span></li></ul></div><div style="display:flex;justify-content:flex-end"><button class="b" data-go="install">Huỷ</button></div>`,
      phone: () => `${sb()}<div class="pb"><div class="sysd" style="margin-top:auto"><p class="t" style="font-weight:700;margin:0">Đang cài Orrery Remote</p><p class="d" style="color:var(--m-muted);font-size:12px;margin:2px 0 8px">Từ: adb (máy tính)</p><div class="prog ind"><i></i></div></div></div>`,
      cap: 'Cài bằng adb, thanh tiến độ theo % thật. Xong tự mở app và đưa mã ghép đôi qua cáp, nên không cần quét QR. Thử state lỗi "usbFail" trên dải trạng thái.',
      next: 'Tự đi tiếp khi cài xong',
    },
    usbFail: {
      label: 'usbFail', step: 1, err: true,
      desk: () => `${wizHead(1, 'install')}<div class="panel"><ul class="chk"><li><span class="ic ok">${I('check', 14)}</span><span class="grow">APK Orrery Remote 4.0.10 có sẵn</span></li><li><span class="ic err">${I('x', 14)}</span><span class="grow">Galaxy A33 từ chối cài<div class="mono">INSTALL_FAILED_USER_RESTRICTED</div></span></li></ul></div>
        <div class="panel row" role="alert"><span class="ic warn">${I('alert', 15)}</span><div class="grow"><p class="t">Điện thoại đang chặn cài qua USB</p><p class="d">Mở Cài đặt, Tuỳ chọn nhà phát triển, bật "Cài qua USB", rồi bấm Thử lại.</p></div></div>
        <div style="display:flex;gap:8px;justify-content:flex-end;flex-wrap:wrap"><button class="b" data-go="qrInstall">${I('qr', 14)} Dùng QR thay thế</button><button class="b pri" data-go="usbInstalling">${I('reset', 14)} Thử lại</button></div>`,
      phone: () => `${sb()}<div class="pb"><p class="ph-title">Tuỳ chọn nhà phát triển</p><div class="ph-card" style="display:flex;align-items:center;gap:8px"><div style="flex:1"><p class="t">Gỡ lỗi USB</p></div><button class="sw" aria-checked="true" aria-label="Gỡ lỗi USB" data-sw></button></div><div class="ph-card flash" style="display:flex;align-items:center;gap:8px"><div style="flex:1"><p class="t">Cài qua USB</p><p class="d">Cho phép cài app qua USB</p></div><button class="sw" aria-checked="false" aria-label="Cài qua USB" data-sw></button></div></div>`,
      cap: 'Lỗi hay gặp trên Xiaomi, Oppo. Nói đúng nguyên nhân, đúng chỗ bật, kèm 2 lối ra: Thử lại hoặc chuyển sang QR.',
      next: 'Thử lại / Dùng QR',
    },
    qrInstall: {
      label: 'qrInstall', step: 1, phoneAct: true,
      desk: () => `${wizHead(1, 'install')}<div class="panel qrbox">${qr(7)}<div style="display:grid;gap:8px;min-width:0"><p class="t" style="font-weight:700;margin:0">Quét bằng camera điện thoại</p><p class="d" style="margin:0;color:var(--m-muted);font-size:13px">Mở trang, bấm "Tải APK". Cài xong bấm "Mở và ghép đôi" trên cùng trang: không cần quét lần 2.</p><div style="display:flex;align-items:center;gap:8px;flex-wrap:wrap">${countdown('qri', 600)}<span class="tag">${I('wifi', 11)} Wi-Fi 192.168.1.24</span></div><p class="mono">http://192.168.1.24:4320/i/8KqZ2v</p><div style="display:flex;gap:6px;flex-wrap:wrap"><button class="b" data-go="install">Ẩn QR</button><button class="b" data-reset-cd="qri">${I('reset', 13)} Tạo link mới</button></div></div></div>`,
      phone: () => `${sb()}<div class="pb"><div class="ph-card mono" style="padding:8px 10px">192.168.1.24:4320/i/8KqZ2v</div><div class="ph-card" style="display:grid;gap:10px;justify-items:center;text-align:center;padding:18px"><span class="ic acc" style="width:56px;height:56px;border-radius:16px">${I('globe', 28)}</span><div><p class="t" style="font-size:15px">Orrery Remote 4.0.10</p><p class="d">24.1 MB, từ máy DESKTOP-JIN</p></div><button class="ph-btn" data-go="qrInstalled">${I('dl', 16)} Tải APK</button></div><p class="d" style="font-size:11.5px;color:var(--m-muted);text-align:center;margin:0">Android có thể hỏi cho phép cài từ Chrome: cho phép 1 lần.</p></div>`,
      cap: 'Trang tải do chính máy tính phục vụ trong mạng nội bộ, link 1 lần, hết hạn 10 phút (vòng đếm ngược chạy thật). Bấm "Tải APK" trên điện thoại.',
      next: 'Điện thoại bấm "Tải APK"',
    },
    qrInstalled: {
      label: 'qrInstalled', step: 1, phoneAct: true,
      desk: () => `${wizHead(1, 'install')}<div class="panel qrbox">${qr(7, true)}<div style="display:grid;gap:8px;min-width:0"><span class="tag ok" style="justify-self:start">${I('check', 11)} Điện thoại đã tải APK</span><p class="d" style="margin:0;color:var(--m-muted);font-size:13px">Chờ cài xong và bấm "Mở và ghép đôi" trên điện thoại.</p></div></div>`,
      phone: () => `${sb()}<div class="pb"><div class="ph-card mono" style="padding:8px 10px">192.168.1.24:4320/i/8KqZ2v</div><div class="ph-card" style="display:grid;gap:10px;padding:16px"><p class="t">${I('check', 13)} Đã tải orrery-remote.apk</p><p class="d">Bước 2: cài xong thì bấm nút dưới, app mở thẳng màn ghép đôi.</p><button class="ph-btn" data-go="pairConfirm">${I('link', 16)} Mở và ghép đôi</button></div></div>`,
      cap: 'Cùng 1 trang làm tiếp bước ghép đôi bằng deep link mang mã 1 lần. Desktop đổi QR sang mờ + "đã tải" để chủ nhân biết điện thoại đã vào.',
      next: 'Điện thoại bấm "Mở và ghép đôi"',
    },
    qrExpired: {
      label: 'qrExpired', step: 1, err: true,
      desk: () => `${wizHead(1, 'install')}<div class="panel qrbox">${qr(7, true)}<div style="display:grid;gap:8px;min-width:0"><span class="tag warn" style="justify-self:start">${I('clock', 11)} Link đã hết hạn</span><p class="d" style="margin:0;color:var(--m-muted);font-size:13px">Link tải chỉ còn hạn 10 phút để người khác trong mạng không tải được.</p><button class="b pri" data-go="qrInstall" style="justify-self:start">${I('reset', 14)} Tạo link mới</button></div></div>`,
      phone: () => `${sb()}<div class="pb"><div class="ph-card"><p class="t">Link đã hết hạn</p><p class="d">Tạo link mới trên máy tính rồi quét lại.</p></div></div>`,
      cap: 'Hết hạn: QR mờ đi, 1 nút chính "Tạo link mới".',
      next: 'Tạo link mới',
    },
    installIos: {
      label: 'installIos', step: 1,
      desk: () => `${wizHead(1, 'off')}${platSeg(true)}<div class="panel qrbox">${qr(11)}<div style="display:grid;gap:6px;min-width:0"><p class="t" style="font-weight:700;margin:0">iPhone dùng web app trên relay của bạn</p><ol style="margin:0;padding-left:18px;font-size:13px"><li>Quét QR, mở bằng Safari</li><li>Bấm Chia sẻ</li><li>Bấm "Thêm vào MH chính", mở Orrery từ màn hình chính</li></ol><p class="mono">https://orrery-relay…/app/</p><button class="b pri" data-go="pairWait" style="justify-self:start">Đã thêm, sang ghép đôi ${I('chev', 13)}</button></div></div>`,
      phone: () => `${sb()}<div class="pb" style="background:var(--m-raised)"><div class="ph-card mono" style="padding:8px 10px">orrery-relay…/app/</div><div class="sysd"><p class="t" style="font-weight:700;margin:0 0 8px">Chia sẻ</p><div class="ph-card" style="display:flex;align-items:center;gap:8px;margin-bottom:6px"><span>${I('copy', 15)}</span>Sao chép</div><div class="ph-card flash" style="display:flex;align-items:center;gap:8px">${I('plus', 15)}<b>Thêm vào MH chính</b></div></div></div>`,
      cap: 'iPhone không cài APK: QR mở web app trên relay Cloudflare của chủ nhân, 3 bước Safari có hình. Thử "iosNoRelay" khi chưa cài relay.',
      next: '"Đã thêm, sang ghép đôi"',
    },
    iosNoRelay: {
      label: 'iosNoRelay', step: 1, err: true,
      desk: () => `${wizHead(1, 'off')}${platSeg(true)}<div class="panel row" role="alert"><span class="ic warn">${I('cloud', 15)}</span><div class="grow"><p class="t">iPhone cần relay Cloudflare</p><p class="d">App iPhone chạy trên relay của chính bạn. Cài 1 lần, khoảng 5 phút, gói miễn phí của Cloudflare.</p></div></div><div style="display:flex;gap:8px;flex-wrap:wrap;justify-content:flex-end"><button class="b" data-toast="Mở hướng dẫn relay">Xem hướng dẫn</button><button class="b pri" data-go="installIos">${I('cloud', 14)} Cài relay</button></div>`,
      phone: () => phHome(false),
      cap: 'Thiếu relay không còn là dòng chữ nhỏ ở mục khác: chặn đúng tại bước iPhone, nói lý do, thời gian, nút cài ngay.',
      next: 'Cài relay',
    },
    pairWait: {
      label: 'pairWait', step: 2, phoneAct: true,
      desk: () => `${wizHead(2, 'install')}<div class="panel qrbox">${qr(23)}<div style="display:grid;gap:8px;min-width:0"><p class="t" style="font-weight:700;margin:0">Mở Orrery Remote, bấm "Thêm máy tính", quét mã này</p><div style="display:flex;align-items:center;gap:8px;flex-wrap:wrap">${countdown('pw', 300)}<span class="m-sub" style="margin:0">Hoặc nhập mã</span><b class="mono" style="font-size:18px;color:var(--m-ink);letter-spacing:.08em">482 913</b></div><p class="d" style="margin:0;color:var(--m-muted);font-size:12.5px">Mã dùng 1 lần. Máy tính vẫn hỏi lại trước khi cho phép.</p></div></div>`,
      phone: () => `${sb()}<div class="pb" style="background:#0b0b0e"><p class="ph-title" style="color:#fff">Thêm máy tính</p><button class="scan" data-go="pairConfirm" aria-label="Quét mã (bấm để giả lập)" style="border:0;cursor:pointer"><div class="fr"></div><p>Hướng camera vào mã QR</p></button><button class="ph-btn sec">Nhập mã 6 số</button></div>`,
      cap: 'Đường iPhone / "đã cài rồi": QR ghép đôi + mã 6 số dự phòng, đếm ngược 5 phút. Bấm khung camera trên điện thoại để giả lập quét.',
      next: 'Điện thoại quét mã',
    },
    pairExpired: {
      label: 'pairExpired', step: 2, err: true,
      desk: () => `${wizHead(2, 'install')}<div class="panel qrbox">${qr(23, true)}<div style="display:grid;gap:8px;min-width:0"><span class="tag warn" style="justify-self:start">${I('clock', 11)} Mã ghép đôi hết hạn</span><button class="b pri" data-go="pairWait" style="justify-self:start">${I('reset', 14)} Tạo mã mới</button></div></div>`,
      phone: () => `${sb()}<div class="pb"><div class="ph-card"><p class="t">Mã đã hết hạn</p><p class="d">Tạo mã mới trên máy tính.</p></div></div>`,
      cap: 'Hết 5 phút: mờ QR, giữ chỗ, 1 nút tạo mã mới.',
      next: 'Tạo mã mới',
    },
    pairConfirm: {
      label: 'pairConfirm', step: 2,
      deskBase: () => `${wizHead(2, 'install')}<div class="panel row"><span class="ic acc">${I('spin', 15)}</span><div class="grow"><p class="t">Galaxy A33 đang xin kết nối</p><p class="d">Qua Wi-Fi 192.168.1.37</p></div></div>`,
      sheet: () => `<div class="sheet" role="dialog" aria-modal="true" aria-labelledby="pc-h"><h4 id="pc-h">Cho phép Galaxy A33 kết nối?</h4><p>Samsung SM-A336E, Android 14, qua Wi-Fi 192.168.1.37. Chỉ cho phép nếu đây là máy của bạn.</p><div class="acts"><button class="b dan" data-go="denied">Từ chối</button><button class="b pri" data-go="paired">${I('check', 14)} Cho phép</button></div></div>`,
      phone: () => phApp(`<div class="ph-card" style="display:grid;gap:10px;justify-items:center;text-align:center;padding:20px">${I('spin', 26)}<p class="t">Chờ máy tính duyệt</p><p class="d">Nhìn màn hình DESKTOP-JIN và bấm "Cho phép".</p></div>`),
      cap: 'Bước duyệt giữ nguyên như bản hiện tại (pha "confirming" của RemotePairPanel): máy tính luôn là bên quyết định.',
      next: 'Cho phép / Từ chối',
    },
    denied: {
      label: 'denied', step: 2, err: true,
      desk: () => `${wizHead(2, 'install')}<div class="panel row" role="alert"><span class="ic err">${I('x', 15)}</span><div class="grow"><p class="t">Đã từ chối Galaxy A33</p><p class="d">Máy này không được kết nối. Muốn thử lại thì tạo mã mới.</p></div><button class="b" data-go="pairWait">Tạo mã mới</button></div>`,
      phone: () => phApp('<div class="ph-card"><p class="t">Máy tính đã từ chối</p><p class="d">Hỏi chủ máy rồi quét mã mới.</p></div>'),
      cap: 'Từ chối: nói rõ đã chặn máy nào, lối ra là tạo mã mới.',
      next: 'Tạo mã mới',
    },
    paired: {
      label: 'paired', step: 2, linked: true, auto: ['testing', 1500],
      desk: () => `${wizHead(2, 'install')}<div class="panel" style="display:grid;justify-items:center;text-align:center;gap:6px;padding:18px"><img src="img/ill03.webp" alt="" width="140" height="140" style="width:140px;height:140px;object-fit:contain"><p class="t" style="font-weight:700;font-size:16px;margin:0">Đã kết nối Galaxy A33</p><p class="d" style="margin:0;color:var(--m-muted);font-size:13px">Đang chuyển sang kiểm tra kết nối…</p></div>`,
      phone: () => phApp(sessionsList),
      cap: 'Khoảnh khắc chính: hành tinh chạy theo quỹ đạo từ máy tính sang điện thoại (600ms, 1 lần), đường nét đứt thành liền màu xanh.',
      next: 'Tự sang Kiểm tra',
    },
    testing: {
      label: 'testing', step: 3, linked: true, auto: ['@testEnd', 1800],
      desk: () => `${wizHead(3, 'install')}<div class="panel"><ul class="chk"><li><span class="ic acc">${I('spin', 14)}</span><span class="grow">Wi-Fi nội bộ</span><span class="v">…</span></li><li><span class="ic acc">${I('spin', 14)}</span><span class="grow">Relay Cloudflare (khi ra ngoài)</span><span class="v">…</span></li><li><span class="ic acc">${I('spin', 14)}</span><span class="grow">Gửi thông báo thử</span><span class="v">…</span></li></ul></div>`,
      phone: () => phApp(sessionsList),
      cap: 'Kiểm 3 đường song song, mỗi dòng có icon + chữ + màu (không dùng màu một mình).',
      next: 'Tự xong',
    },
    testDone: {
      label: 'testDone', step: 3, linked: true,
      desk: () => `${wizHead(3, 'install')}<div class="panel"><ul class="chk"><li><span class="ic ok">${I('check', 14)}</span><span class="grow">Wi-Fi nội bộ</span><span class="v">18 ms</span></li><li><span class="ic ok">${I('check', 14)}</span><span class="grow">Relay Cloudflare</span><span class="v">142 ms</span></li><li><span class="ic ok">${I('check', 14)}</span><span class="grow">Đã gửi thông báo thử</span></li></ul></div><div class="panel row"><span class="ic">${I('bell', 15)}</span><p class="grow t" style="margin:0">Điện thoại có hiện thông báo không?</p><button class="b" data-go="pushHelp">Không thấy</button><button class="b pri" data-go="done">Có, xong</button></div>`,
      phone: () => phHome(true, `<div class="notif"><div class="h">${orrIcon} Orrery Remote, bây giờ</div><p class="t">Thông báo thử từ DESKTOP-JIN</p><p class="d">Thấy tin này là push đã chạy.</p></div>`),
      cap: 'Hỏi người dùng xác nhận push thật sự hiện, vì máy chủ push trả OK chưa chắc điện thoại hiện (tối ưu pin, tắt thông báo).',
      next: 'Có, xong / Không thấy',
    },
    testWarn: {
      label: 'testWarn', step: 3, linked: true, err: true,
      desk: () => `${wizHead(3, 'install')}<div class="panel"><ul class="chk"><li><span class="ic ok">${I('check', 14)}</span><span class="grow">Wi-Fi nội bộ</span><span class="v">21 ms</span></li><li><span class="ic warn">${I('alert', 14)}</span><span class="grow">Chưa có relay: ra khỏi Wi-Fi nhà sẽ mất kết nối</span><button class="b" data-toast="Mở cài relay">Cài relay</button></li><li><span class="ic ok">${I('check', 14)}</span><span class="grow">Đã gửi Web Push thử</span></li></ul></div><div style="display:flex;justify-content:flex-end;gap:8px"><button class="b pri" data-go="done">Để sau, xong</button></div>`,
      phone: () => phHome(true, `<div class="notif"><div class="h">${orrIcon} Orrery, bây giờ</div><p class="t">Thông báo thử từ DESKTOP-JIN</p></div>`),
      cap: 'Một phần chưa ổn vẫn cho xong; chỉ rõ hệ quả ("ra khỏi Wi-Fi nhà sẽ mất kết nối") và nút sửa ngay dòng đó.',
      next: 'Cài relay / Để sau',
    },
    pushHelp: {
      label: 'pushHelp', step: 3, linked: true, err: true,
      desk: () => `${wizHead(3, 'install')}<div class="panel"><div class="row"><span class="ic warn">${I('bell', 15)}</span><div class="grow"><p class="t">Push đã gửi nhưng điện thoại không hiện</p><p class="d">Thường do 1 trong 2 chỗ sau trên Galaxy A33:</p></div></div><ol class="chk" style="list-style:none"><li><span class="ic">1</span><span class="grow">Cài đặt, Ứng dụng, Orrery Remote, Thông báo: bật</span><button class="b" data-toast="Đã mở cài đặt thông báo trên Galaxy A33 qua adb">Mở trên điện thoại</button></li><li><span class="ic">2</span><span class="grow">Pin: chọn "Không hạn chế" cho Orrery Remote</span><button class="b" data-toast="Đã mở cài đặt pin trên Galaxy A33 qua adb">Mở trên điện thoại</button></li></ol></div><div style="display:flex;justify-content:flex-end;gap:8px"><button class="b" data-go="done">Bỏ qua</button><button class="b pri" data-go="testDone">${I('send', 14)} Gửi lại</button></div>`,
      phone: () => `${sb()}<div class="pb"><p class="ph-title">Orrery Remote</p><div class="ph-card flash" style="display:flex;align-items:center;gap:8px"><div style="flex:1"><p class="t">Hiện thông báo</p></div><button class="sw" aria-checked="false" aria-label="Hiện thông báo" data-sw></button></div><div class="ph-card"><p class="t">Pin</p><p class="d">Tối ưu hoá</p></div></div>`,
      cap: 'Máy đang cắm adb thì nút "Mở trên điện thoại" mở thẳng đúng trang cài đặt, khỏi tự tìm.',
      next: 'Gửi lại / Bỏ qua',
    },
    done: {
      label: 'done', step: 0, linked: true, toast: 'Đã kết nối Galaxy A33',
      desk: () => `<div><p class="m-h">Điện thoại và thiết bị</p><p class="m-sub">1 điện thoại đang kết nối.</p></div>
        <div class="panel hero-c"><img src="img/ill03.webp" alt="" width="120" height="120"><div><p class="t" style="font-weight:700;font-size:15px;margin:0">Điện thoại đã sẵn sàng</p><div class="tags"><span class="tag ok">${I('wifi', 11)} Wi-Fi 18 ms</span><span class="tag ok">${I('cloud', 11)} Relay</span><span class="tag ok">${I('bell', 11)} Push</span></div><div style="display:flex;gap:6px;margin-top:10px;flex-wrap:wrap"><button class="b" data-go="install">${I('plus', 14)} Kết nối thêm</button><button class="b" data-go="testing">Kiểm lại</button></div></div></div>
        <div class="panel"><div class="row new flash"><span class="ic ok">${I('phone', 15)}</span><div class="grow"><p class="t">Galaxy A33</p><div class="tags"><span class="tag acc">Remote 4.0.10</span><span class="tag">${I('usb', 11)} USB adb</span></div></div><span class="tag ok">${I('check', 11)} Online</span></div><div class="row"><span class="ic">${I('bot', 15)}</span><div class="grow"><p class="t">Medium_Phone_API_35</p><div class="tags"><span class="tag">Emulator</span><span class="tag">Đang tắt</span></div></div></div></div>${ADV}`,
      phone: () => phApp(sessionsList),
      cap: 'Xong: thẻ trên cùng tóm trạng thái 3 đường, máy mới nằm đầu danh sách thiết bị (nháy nền 1 lần), toast xác nhận.',
      next: '',
    },
  }

  const SC = {
    usb: { flow: ['off', 'consent', 'install', 'usbInstalling', 'pairConfirm', 'paired', 'testing', 'testDone', 'done'], errs: ['usbFail', 'denied', 'pushHelp'], first: 'install', testEnd: 'testDone' },
    qr: { flow: ['off', 'consent', 'install', 'qrInstall', 'qrInstalled', 'pairConfirm', 'paired', 'testing', 'testDone', 'done'], errs: ['qrExpired', 'denied', 'pushHelp'], first: 'install', testEnd: 'testDone' },
    ios: { flow: ['off', 'consent', 'installIos', 'pairWait', 'pairConfirm', 'paired', 'testing', 'testWarn', 'done'], errs: ['iosNoRelay', 'pairExpired', 'denied'], first: 'installIos', testEnd: 'testWarn' },
  }

  const cn = { sc: 'usb', st: 'off', prevStep: 0, timer: 0, play: false, playT: 0, dark: false }
  const winEl = $('#cn-win'), scrEl = $('#cn-scr'), capEl = $('#cn-cap'), stageEl = $('#cn-stage'), orbitEl = $('#cn-orbit')

  function resolve(id) {
    if (id === '@install') return SC[cn.sc].first
    if (id === '@testEnd') return SC[cn.sc].testEnd
    return id
  }

  function renderStates() {
    const s = SC[cn.sc]
    $('#cn-states').innerHTML = [...s.flow, ...s.errs].map((id) => `<button class="${CN[id].err ? 'err' : ''} ${id === cn.st ? 'on' : ''}" data-st="${id}" aria-pressed="${id === cn.st}">${id}</button>`).join('')
  }

  function placeOrbit() {
    if (getComputedStyle(orbitEl).display === 'none') return
    const sr = stageEl.getBoundingClientRect()
    const a = winEl.getBoundingClientRect(), b = $('#cn-phone').getBoundingClientRect()
    const x1 = a.right - sr.left - 90, y1 = a.top - sr.top
    const x2 = b.left - sr.left + 70, y2 = b.top - sr.top + 8
    const d = `M${x1},${y1} C${x1 + 30},${y1 - 34} ${x2 - 40},${y2 - 40} ${x2},${y2}`
    $('#cn-arc').setAttribute('d', d)
    $('.planet', orbitEl).style.offsetPath = `path('${d}')`
  }

  function go(id, opts = {}) {
    id = resolve(id)
    const s = CN[id]
    if (!s) return
    clearTimeout(cn.timer)
    const forward = (s.step || 0) >= cn.prevStep
    const prevLinked = CN[cn.st]?.linked
    cn.st = id
    const deskHtml = s.desk ? s.desk(cn.sc) : s.deskBase ? s.deskBase(cn.sc) : CN[s.base].desk(cn.sc)
    const sheet = s.sheet ? `<div class="sheet-wrap">${s.sheet()}</div>` : ''
    const toast = s.toast ? `<div class="toast" role="status">${I('check', 14)} ${s.toast}<span class="bar"></span></div>` : ''
    winEl.innerHTML = win('Orrery, Settings', `<div class="swap ${forward ? '' : 'back'}" style="display:flex;flex-direction:column;gap:12px">${deskHtml}</div>${sheet}${toast}`)
    scrEl.innerHTML = s.phone ? s.phone(cn.sc) : ''
    capEl.innerHTML = `<b>${id}</b><span>${s.cap}</span>`
    cn.prevStep = s.step || 0
    orbitEl.classList.toggle('linked', !!s.linked)
    orbitEl.classList.remove('go')
    if (s.linked && !prevLinked) { void orbitEl.getBoundingClientRect(); orbitEl.classList.add('go') }
    renderStates()
    setView(s.phoneAct ? 'phone' : 'win', true)
    tickCd()
    requestAnimationFrame(placeOrbit)
    if (id === 'usbInstalling') animateProgress()
    if (id === 'qrInstall' && opts.fresh !== false) cdEnds.delete('qri')
    if (id === 'pairWait') cdEnds.delete('pw')
    if (s.auto && !opts.manual) cn.timer = setTimeout(() => go(s.auto[0]), s.auto[1])
    if (opts.focus) { const b = $('button:not([disabled])', winEl); if (b && document.activeElement && winEl.contains(document.activeElement) === false && opts.focus === 'win') b.focus({ preventScroll: true }) }
  }

  function setView(v, auto) {
    stageEl.dataset.view = v
    $$('#cn-view [data-v]').forEach((b) => {
      const on = b.dataset.v === v
      b.classList.toggle('on', on)
      b.setAttribute('aria-selected', String(on))
      const p = $('.ping', b)
      if (p) p.hidden = on || !auto
    })
  }
  $('#cn-view').addEventListener('click', (e) => {
    const b = e.target.closest('[data-v]')
    if (b) setView(b.dataset.v, false)
  })

  function animateProgress() {
    const bar = $('[data-grow]', winEl), pct = $('[data-pct]', winEl)
    if (!bar) return
    let p = 0
    const step = () => {
      if (cn.st !== 'usbInstalling') return
      p = Math.min(100, p + 9 + Math.random() * 10)
      bar.style.setProperty('--p', p / 100)
      pct.textContent = `${Math.round(p)}%`
      if (p < 100) setTimeout(step, 230)
    }
    setTimeout(step, 150)
  }

  function stopPlay() {
    cn.play = false
    clearTimeout(cn.playT)
    setPlayBtn()
  }
  function setPlayBtn() {
    const b = $('#cn-play')
    b.innerHTML = cn.play ? `${I('pause', 14)} Dừng` : `${I('play', 14)} Chạy tự động`
    b.setAttribute('aria-pressed', String(cn.play))
  }
  function playStep() {
    if (!cn.play) return
    const flow = SC[cn.sc].flow
    const i = flow.indexOf(cn.st)
    if (i < 0 || i >= flow.length - 1) { stopPlay(); return }
    go(flow[i + 1], { manual: true })
    const wait = { consent: 1600, usbInstalling: 2600, paired: 1700, testing: 1800 }[cn.st] || 2300
    cn.playT = setTimeout(playStep, wait)
  }

  $('#cn-ctl').addEventListener('click', (e) => {
    const b = e.target.closest('button')
    if (!b) return
    if (b.dataset.sc) {
      $$('#cn-ctl [data-sc]').forEach((x) => x.classList.toggle('on', x === b))
      cn.sc = b.dataset.sc
      stopPlay()
      go('off', { manual: true })
    } else if (b.id === 'cn-play') {
      if (cn.play) { stopPlay(); return }
      cn.play = true
      setPlayBtn()
      if (cn.st === 'done' || !SC[cn.sc].flow.includes(cn.st)) go('off', { manual: true })
      cn.playT = setTimeout(playStep, 900)
    } else if (b.id === 'cn-reset') {
      stopPlay()
      go('off', { manual: true })
    } else if (b.id === 'cn-dark') {
      cn.dark = !cn.dark
      $$('.app').forEach((a) => a.toggleAttribute('data-dark', cn.dark))
      b.setAttribute('aria-pressed', String(cn.dark))
      setDarkBtn()
    }
  })
  function setDarkBtn() { $('#cn-dark').innerHTML = cn.dark ? `${I('sun', 14)} Mock sáng` : `${I('moon', 14)} Mock tối` }
  $('#cn-reset').innerHTML = `${I('reset', 14)} Làm lại`
  setDarkBtn()
  setPlayBtn()

  $('#cn-states').addEventListener('click', (e) => {
    const b = e.target.closest('[data-st]')
    if (!b) return
    stopPlay()
    go(b.dataset.st, { manual: true })
  })

  let toastT = 0
  function miniToast(host, text, undo) {
    $$('.toast', host).forEach((t) => t.remove())
    clearTimeout(toastT)
    const t = document.createElement('div')
    t.className = 'toast'
    t.setAttribute('role', 'status')
    t.innerHTML = `${I('check', 14)} <span>${esc(text)}</span>${undo ? `<button class="b" data-undo>${I('undo', 13)} Hoàn tác</button><span class="bar"></span>` : ''}`
    host.appendChild(t)
    toastT = setTimeout(() => { t.remove(); if (undo) undo.commit?.() }, undo ? 5000 : 2600)
    if (undo) $('[data-undo]', t).addEventListener('click', () => { clearTimeout(toastT); t.remove(); undo.revert() })
  }

  function commonClicks(e, host) {
    const sw = e.target.closest('[data-sw]')
    if (sw) { sw.setAttribute('aria-checked', String(sw.getAttribute('aria-checked') !== 'true')); return true }
    const adv = e.target.closest('[data-toggle-adv]')
    if (adv) {
      const box = adv.parentElement
      const open = !box.hasAttribute('open-x')
      box.toggleAttribute('open-x', open)
      adv.setAttribute('aria-expanded', String(open))
      return true
    }
    const t = e.target.closest('[data-toast]')
    if (t) { miniToast(host, t.dataset.toast); return true }
    return false
  }

  stageEl.addEventListener('click', (e) => {
    const winMain = $('.win-main', winEl)
    if (commonClicks(e, winMain)) return
    const r = e.target.closest('[data-reset-cd]')
    if (r) { cdEnds.delete(r.dataset.resetCd); go(cn.st, { manual: true }); return }
    const g = e.target.closest('[data-go]')
    if (g && !g.disabled) { stopPlay(); go(g.dataset.go) }
  })

  addEventListener('resize', () => requestAnimationFrame(placeOrbit))
  go('off', { manual: true })

  const stRows = Object.entries(CN).map(([id, s]) => {
    const dDesk = { off: 'Thẻ "Chưa có điện thoại", nút Kết nối, máy adb sẵn có, Nâng cao gập', consent: 'Hộp thoại đồng ý: quyền điện thoại có, secret không rời máy', install: 'Bước 1: Android / iPhone, 3 cách cài, nút "Đã cài rồi"', usbInstalling: '3 dòng tiến trình, % cài thật', usbFail: 'Lỗi + mã lỗi + chỗ bật + Thử lại / Dùng QR', qrInstall: 'QR tải + đếm ngược 10 phút + địa chỉ', qrInstalled: 'QR mờ + "Điện thoại đã tải APK"', qrExpired: 'QR mờ + Tạo link mới', installIos: 'QR web app + 3 bước Safari', iosNoRelay: 'Cảnh báo thiếu relay + Cài relay', pairWait: 'QR ghép đôi + mã 6 số + đếm ngược 5 phút', pairExpired: 'QR mờ + Tạo mã mới', pairConfirm: 'Hộp thoại Cho phép / Từ chối', denied: 'Đã từ chối + Tạo mã mới', paired: 'Minh hoạ đã kết nối, hành tinh chạy quỹ đạo', testing: '3 dòng kiểm đang chạy', testDone: '3 dòng xong + hỏi "có hiện thông báo không"', testWarn: 'Dòng relay cảnh báo + Cài relay', pushHelp: '2 chỗ sửa + Mở trên điện thoại', done: 'Thẻ trạng thái, máy mới đầu danh sách, toast' }[id]
    const dPhone = { off: 'Màn hình chính', consent: 'Màn hình chính', install: 'Màn hình chính', usbInstalling: 'Hộp "Đang cài"', usbFail: 'Tuỳ chọn nhà phát triển', qrInstall: 'Trang tải + nút Tải APK', qrInstalled: 'Nút "Mở và ghép đôi"', qrExpired: 'Link hết hạn', installIos: 'Bảng Chia sẻ Safari', iosNoRelay: 'Màn hình chính', pairWait: 'Camera quét', pairExpired: 'Mã hết hạn', pairConfirm: 'Chờ máy tính duyệt', denied: 'Bị từ chối', paired: 'Danh sách session', testing: 'Danh sách session', testDone: 'Thông báo thử', testWarn: 'Thông báo thử', pushHelp: 'Cài đặt thông báo app', done: 'Danh sách session' }[id]
    return `<tr><td><code>${id}</code>${s.err ? ' <span class="pill p-warn">lỗi</span>' : ''}</td><td>${dDesk}</td><td>${dPhone}</td><td>${s.next || 'Kết thúc'}</td></tr>`
  })
  $('#st-table').innerHTML = stRows.join('')

  const DV0 = [
    { id: 'a33', name: 'Galaxy A33', model: 'Samsung SM-A336E, Android 14', kind: 'phone', remote: true, adb: 'USB', ver: '4.0.8', st: ['ok', 'Online, Wi-Fi 18 ms'], push: true, act: 'update' },
    { id: 'px8', name: 'Pixel 8', model: 'Google, Android 15', kind: 'phone', remote: true, ver: '4.0.10', st: ['ok', 'Online qua relay, 140 ms'], push: true },
    { id: 'ip15', name: 'iPhone 15', model: 'iOS 18, web app', kind: 'phone', remote: true, web: true, st: ['', 'Offline, 2 giờ trước'], push: false },
    { id: 'emu', name: 'Medium_Phone_API_35', model: 'Emulator, Android 15', kind: 'emu', st: ['', 'Đang tắt'], act: 'boot' },
  ]
  const NEW_DEV = { id: 'rn12', name: 'Redmi Note 12', model: 'Xiaomi, Android 13', kind: 'phone', adb: 'USB', st: ['wait', 'Chưa cài Remote'], act: 'install', fresh: true }
  const dv = { list: DV0.map((d) => ({ ...d })), open: '', filter: 'all', empty: false, ask: '', busy: {} }
  const dvWin = $('#dv-win')

  function dvMatch(d) {
    if (dv.filter === 'remote') return d.remote
    if (dv.filter === 'adb') return !!d.adb
    if (dv.filter === 'emu') return d.kind === 'emu'
    return true
  }
  function dvRow(d) {
    const tags = []
    if (d.remote) tags.push(d.web ? '<span class="tag acc">Web app</span>' : `<span class="tag ${d.act === 'update' ? 'warn' : 'acc'}">Remote ${d.ver}${d.act === 'update' ? ', cũ hơn desktop' : ''}</span>`)
    if (d.adb) tags.push(`<span class="tag">${I('usb', 11)} ${d.adb} adb</span>`)
    if (d.kind === 'emu') tags.push('<span class="tag">Emulator</span>')
    const busy = dv.busy[d.id]
    let act = ''
    if (busy) act = `<button class="b" disabled>${I('spin', 13)} ${busy}</button>`
    else if (d.act === 'update') act = `<button class="b pri" data-dv="update" data-id="${d.id}">${I('dl', 13)} Cập nhật 4.0.10</button>`
    else if (d.act === 'boot') act = `<button class="b" data-dv="boot" data-id="${d.id}">${I('play', 13)} Khởi động</button>`
    else if (d.act === 'install') act = `<button class="b pri" data-dv="install" data-id="${d.id}">${I('dl', 13)} Cài Remote</button>`
    const icon = d.kind === 'emu' ? 'bot' : 'phone'
    const open = dv.open === d.id
    let det = ''
    if (open) {
      if (dv.ask === d.id) {
        det = `<div class="det" role="group" aria-label="Xác nhận thu hồi"><div class="line">${I('alert', 14)} Thu hồi ${esc(d.name)}? Điện thoại sẽ phải ghép đôi lại.</div><div class="acts"><button class="b dan" data-dv="revokeYes" data-id="${d.id}">Thu hồi</button><button class="b" data-dv="revokeNo" data-id="${d.id}">Huỷ</button></div></div>`
      } else {
        det = `<div class="det">${d.remote ? `<div class="line"><span style="flex:1">Gửi thông báo tới máy này</span><button class="sw" role="switch" aria-checked="${d.push}" aria-label="Thông báo" data-sw></button></div>` : ''}<div class="line">${esc(d.model)}</div><div class="acts">${d.remote ? `<button class="b" data-toast="Đã gửi thông báo thử tới ${esc(d.name)}">${I('send', 13)} Gửi thử</button>` : ''}${d.adb ? `<button class="b" data-toast="Đã mở app trên ${esc(d.name)}">${I('play', 13)} Mở app</button><button class="b" data-toast="Đã chụp màn ${esc(d.name)}">${I('cam', 13)} Chụp màn</button>` : ''}<button class="b" data-toast="Đổi tên (minh hoạ)">${I('pencil', 13)} Đổi tên</button>${d.remote ? `<button class="b dan" data-dv="revoke" data-id="${d.id}">Thu hồi</button>` : ''}</div></div>`
      }
    }
    return `<div class="dv-item" data-item="${d.id}"><div class="row ${d.fresh ? 'new' : ''}" data-dv="open" data-id="${d.id}" role="button" tabindex="0" aria-expanded="${open}"><span class="ic ${d.st[0] === 'ok' ? 'ok' : ''}">${I(icon, 15)}</span><div class="grow"><p class="t">${esc(d.name)}</p><div class="tags">${tags.join('')}</div></div><span class="tag ${d.st[0]}">${d.st[0] === 'ok' ? I('check', 11) : d.st[0] === 'wait' ? I('clock', 11) : ''} ${d.st[1]}</span>${act}${I('chev', 15, 'chev')}</div>${det}</div>`
  }
  function dvRender() {
    const list = dv.empty ? [] : dv.list
    const shown = list.filter(dvMatch)
    const counts = { all: list.length, remote: list.filter((d) => d.remote).length, adb: list.filter((d) => d.adb).length, emu: list.filter((d) => d.kind === 'emu').length }
    const F = [['all', 'Tất cả'], ['remote', 'App Remote'], ['adb', 'adb'], ['emu', 'Emulator']]
    let body = `<div style="display:flex;align-items:flex-start;gap:10px;flex-wrap:wrap"><div style="flex:1 1 200px;min-width:0"><p class="m-h">Thiết bị</p><p class="m-sub">${list.length} thiết bị; điện thoại Remote và adb gộp theo máy.</p></div><button class="b pri" data-jump="ket-noi">${I('plus', 14)} Kết nối điện thoại</button></div>`
    if (!list.length) {
      body += `<div class="panel empty-m"><img src="img/ill04.webp" alt="" width="110" height="110"><p class="t" style="font-weight:700;margin:0">Chưa có thiết bị nào</p><p class="d" style="margin:0;color:var(--m-muted);font-size:13px;max-width:340px">Cắm cáp USB một máy Android, hoặc kết nối điện thoại qua QR. Máy mới tự hiện ở đây.</p><div style="display:flex;gap:8px;flex-wrap:wrap;justify-content:center"><button class="b" data-toast="Mở ghép adb qua Wi-Fi">${I('wifi', 13)} Ghép adb qua Wi-Fi</button><button class="b pri" data-jump="ket-noi">${I('phone', 13)} Kết nối điện thoại</button></div></div>`
    } else {
      body += `<div class="filter-m" role="group" aria-label="Lọc thiết bị">${F.map(([k, n]) => `<button class="${dv.filter === k ? 'on' : ''}" data-dvf="${k}" aria-pressed="${dv.filter === k}">${n} (${counts[k]})</button>`).join('')}</div><div class="panel devs">${shown.map(dvRow).join('') || '<div class="row"><p class="d">Không có thiết bị trong nhóm này.</p></div>'}</div>`
    }
    body += ADV.replace('Nâng cao', 'Nâng cao điện thoại')
    dvWin.innerHTML = win('Orrery, Settings', body)
    $('#dv-cap').innerHTML = dv.empty ? '<b>Trạng thái trống</b><span>Lời mời hành động thay vì khung trắng: 2 cách thêm máy.</span>' : '<b>Gộp dòng</b><span>Galaxy A33 vừa cắm USB vừa cài Remote nên chỉ có 1 dòng, 2 tag. App trên máy cũ hơn desktop thì nút "Cập nhật" hiện ngay dòng đó (cài qua adb).</span>'
    dv.list.forEach((d) => { d.fresh = false })
  }
  dvWin.addEventListener('click', (e) => {
    const main = $('.win-main', dvWin)
    if (commonClicks(e, main)) return
    const j = e.target.closest('[data-jump]')
    if (j) { $('#' + j.dataset.jump).scrollIntoView({ behavior: RM.matches ? 'auto' : 'smooth' }); return }
    const f = e.target.closest('[data-dvf]')
    if (f) { dv.filter = f.dataset.dvf; dvRender(); return }
    const b = e.target.closest('[data-dv]')
    if (!b) return
    const d = dv.list.find((x) => x.id === b.dataset.id)
    const a = b.dataset.dv
    if (a === 'open') { dv.open = dv.open === d.id ? '' : d.id; dv.ask = ''; dvRender(); return }
    e.stopPropagation()
    if (a === 'update' || a === 'install' || a === 'boot') {
      dv.busy[d.id] = { update: 'Đang cập nhật', install: 'Đang cài', boot: 'Đang khởi động' }[a]
      dvRender()
      setTimeout(() => {
        delete dv.busy[d.id]
        if (a === 'update') { d.ver = '4.0.10'; d.act = ''; miniToast($('.win-main', dvWin), `Đã cập nhật ${d.name} lên 4.0.10`) }
        if (a === 'install') { d.remote = true; d.ver = '4.0.10'; d.act = ''; d.st = ['wait', 'Chờ ghép đôi']; miniToast($('.win-main', dvWin), `Đã cài, duyệt ${d.name} khi app xin kết nối`) }
        if (a === 'boot') { d.act = ''; d.st = ['ok', 'Online'] }
        dvRender()
      }, 1600)
      return
    }
    if (a === 'revoke') { dv.ask = d.id; dvRender(); return }
    if (a === 'revokeNo') { dv.ask = ''; dvRender(); return }
    if (a === 'revokeYes') {
      const el = $(`[data-item="${d.id}"] .row`, dvWin)
      const idx = dv.list.indexOf(d)
      const finish = () => {
        dv.list.splice(idx, 1)
        dv.open = ''
        dv.ask = ''
        dvRender()
        miniToast($('.win-main', dvWin), `Đã thu hồi ${d.name}`, { revert: () => { d.fresh = true; dv.list.splice(idx, 0, d); dvRender() } })
      }
      if (RM.matches) finish()
      else { el.classList.add('gone'); setTimeout(finish, 170) }
    }
  })
  dvWin.addEventListener('keydown', (e) => {
    if ((e.key === 'Enter' || e.key === ' ') && e.target.matches('[data-dv="open"]')) { e.preventDefault(); e.target.click() }
  })
  $('#dv-ctl').addEventListener('click', (e) => {
    const b = e.target.closest('button')
    if (!b) return
    if (b.id === 'dv-plug') {
      if (dv.list.some((d) => d.id === NEW_DEV.id)) { miniToast($('.win-main', dvWin), 'Redmi Note 12 đã có trong danh sách'); return }
      dv.empty = false
      dv.list.unshift({ ...NEW_DEV })
      dvRender()
      miniToast($('.win-main', dvWin), 'Vừa cắm Redmi Note 12 qua USB')
    } else if (b.id === 'dv-empty') {
      dv.empty = !dv.empty
      b.setAttribute('aria-pressed', String(dv.empty))
      dvRender()
    } else if (b.id === 'dv-reset') {
      dv.list = DV0.map((d) => ({ ...d }))
      dv.open = ''; dv.ask = ''; dv.filter = 'all'; dv.empty = false
      $('#dv-empty').setAttribute('aria-pressed', 'false')
      dvRender()
    }
  })
  $('#dv-plug').innerHTML = `${I('usb', 14)} Cắm thêm 1 máy`
  $('#dv-empty').innerHTML = `${I('phone', 14)} Xem trạng thái trống`
  $('#dv-reset').innerHTML = `${I('reset', 14)} Làm lại`
  dvRender()

  const FT = window.NF.feats
  const EX = window.NF.extra
  const ALL = [...FT, ...EX]
  const valPill = (v) => `<span class="pill ${v === 'cao' ? 'p-c2' : 'p-off'}">Giá trị ${v}</span>`
  $('#ft-list').innerHTML = FT.map((f) => `<a class="feat" href="#${f.to}" data-g="${f.g}"><div class="fh"><img src="img/${f.icon}.webp" alt="" width="52" height="52" loading="lazy"><div><span class="fid">${f.id}</span><strong>${esc(f.t)}</strong></div></div><p>${esc(f.d)}</p><div class="pills">${valPill(f.v)}<span class="pill p-c6">Công sức ${f.e}</span>${f.warn ? '<span class="pill p-warn">Cần ghi ngoại lệ</span>' : ''}</div></a>`).join('')
  $('#ex-list').innerHTML = EX.map((f) => `<a class="feat" href="#${f.to}"><div class="fh"><img src="img/${f.icon}.webp" alt="" width="52" height="52" loading="lazy"><div><span class="fid">${f.id}</span><strong>${esc(f.t)}</strong></div></div><p>${esc(f.d)}</p><div class="pills">${valPill(f.v)}<span class="pill p-c6">Công sức ${f.e}</span><span class="pill p-c3">${f.wave}</span></div></a>`).join('')
  $('#ex-rv').innerHTML = window.NF.extraReview.map(([id, v, e, r, k]) => {
    const f = EX.find((x) => x.id === id)
    return `<tr><td><b>${id}</b> ${esc(f.t)}</td><td>${f.wave}</td><td>${v}</td><td>${e}</td><td>${r}</td><td class="hide-sm">${k}</td></tr>`
  }).join('')
  $('#ft-filter').addEventListener('click', (e) => {
    const b = e.target.closest('[data-g]')
    if (!b) return
    $$('#ft-filter button').forEach((x) => { x.classList.toggle('on', x === b); x.setAttribute('aria-pressed', String(x === b)) })
    $$('#ft-list .feat').forEach((c) => c.classList.toggle('hide', b.dataset.g !== 'all' && c.dataset.g !== b.dataset.g))
  })
  $$('#ft-filter button').forEach((x) => x.setAttribute('aria-pressed', String(x.classList.contains('on'))))

  $('#rv-table').innerHTML = window.NF.review.map(([id, v, e, r, k]) => {
    const f = FT.find((x) => x.id === id)
    return `<tr><td><b>${id}</b> ${esc(f.t)}</td><td>${v}</td><td>${e}</td><td>${r}</td><td class="hide-sm">${k}</td></tr>`
  }).join('')

  const demos = []
  const card = (id, why, stage, acts) => {
    const f = ALL.find((x) => x.id === id)
    return `<article class="demo" id="demo-${id.toLowerCase()}" data-demo="${id}"><div class="demo-h"><strong><img src="img/${f.icon}.webp" alt="" width="36" height="36" style="width:36px;height:36px">${id} ${esc(f.t)}</strong>${f.warn ? '<span class="pill p-warn">Cần ghi ngoại lệ</span>' : `<span class="pill p-c6">Công sức ${f.e}</span>`}</div><p class="why">${why}</p><div class="demo-stage" data-stage>${stage}</div><div class="acts" data-acts>${acts}</div></article>`
  }
  const miniPhone = (inner) => `<div class="phone sm"><i class="cam"></i><div class="scr">${inner}</div></div>`
  const miniWin = (title, inner) => `<div class="win mini-win"><div class="win-bar"><i></i><i></i><i></i><span>${title}</span></div><div class="win-main">${inner}</div></div>`

  const D = {}
  D.P1 = {
    s: 'ask',
    render(s) {
      const term = s === 'ask' ? '<span class="p">Claude</span> Do you want to make this edit to auth.ts?<br>❯ 1. Yes<br>  2. Yes, allow all edits this session<br>  3. No, tell Claude what to do' : '<span class="p">Claude</span> Do you want to make this edit to auth.ts?<br>❯ 1. Yes<br><br><span style="color:#4cc38a">Đã nhận "1" từ Galaxy A33</span><br>Editing auth.ts…'
      const n = s === 'ask'
        ? `<div class="notif"><div class="h">${orrIcon} Orrery, api-server, 9:41</div><p class="t">Claude hỏi: sửa auth.ts?</p><p class="d">Do you want to make this edit to auth.ts?</p><div class="acts"><button class="pri" data-p1="1">1 Yes</button><button data-p1="2">2 Allow all</button><button data-p1="3">3 No</button></div></div>`
        : `<div class="notif"><div class="h">${orrIcon} Orrery, api-server</div><p class="t">${I('check', 12)} Đã gõ "1" vào api-server</p></div>`
      return miniPhone(`${sb()}<div class="pb" style="background:var(--m-raised)">${n}${appsGrid(true)}</div>`) + miniWin('api-server, terminal', `<div class="term">${term}</div>`)
    },
    acts: () => `<button class="chipbtn ghost" data-reset>${I('reset', 14)} Làm lại</button>`,
    on(e, el) { const b = e.target.closest('[data-p1]'); if (b) { this.s = 'sent'; return true } },
  }
  D.P2 = {
    n: 3,
    render() {
      const items = [['api-server', 'Claude hỏi quyền sửa file', 'var(--m-wait)'], ['web-app', 'Codex chờ chọn 1/2/3', 'var(--m-wait)'], ['docs', 'Xong, 12 file đổi', 'var(--m-done)']].slice(3 - this.n)
      return miniPhone(`${sb()}<div class="pb" style="background:var(--m-raised)"><div class="widget" role="img" aria-label="Widget Orrery: ${this.n} session chờ bạn"><div style="display:flex;align-items:center;gap:6px;font-size:11px;color:var(--m-muted)">${orrIcon} Orrery</div><div class="big">${this.n}</div><div style="font-size:11.5px;color:var(--m-muted);margin-top:-4px">chờ bạn</div>${items.slice(0, 2).map(([a, b, c]) => `<div class="it"><i class="dot" style="background:${c}"></i><span><b style="color:var(--m-ink)">${a}</b> ${b}</span></div>`).join('')}</div>${appsGrid(true)}</div>`)
    },
    acts: () => `<button class="chipbtn ghost" data-p2="-">${I('check', 14)} Mô phỏng: 1 session được trả lời</button><button class="chipbtn ghost" data-reset>${I('reset', 14)} Làm lại</button>`,
    on(e) { const b = e.target.closest('[data-p2]'); if (b) { this.n = Math.max(0, this.n - 1); return true } },
  }
  D.P3 = {
    s: 'run', t0: 0,
    render() {
      const n = this.s === 'run'
        ? `<div class="notif"><div class="h">${orrIcon} Orrery, đang chạy</div><p class="t">Claude đang làm web-app <span class="count" data-p3t>4:12</span></p><p class="d">Edit src/App.vue</p><div class="prog ind"><i></i></div></div>`
        : `<div class="notif"><div class="h">${orrIcon} Orrery, vừa xong</div><p class="t">${I('check', 12)} web-app xong sau 6:40</p><p class="d">12 file đổi, chờ bạn review</p><div class="acts"><button class="pri">Mở pane</button><button>Xem thay đổi</button></div></div>`
      return miniPhone(`${sb()}<div class="pb" style="background:var(--m-raised)">${n}<div class="ph-card"><p class="t">Thông báo cố định</p><p class="d">Không vuốt bỏ được khi đang chạy; tự tắt khi xong.</p></div></div>`)
    },
    acts: () => `<button class="chipbtn ghost" data-p3>${I('check', 14)} Mô phỏng: agent xong</button><button class="chipbtn ghost" data-reset>${I('reset', 14)} Làm lại</button>`,
    on(e) { if (e.target.closest('[data-p3]')) { this.s = 'done'; return true } },
  }
  D.P4 = {
    s: 'idle', text: '', att: false,
    render() {
      const rec = this.s === 'rec'
      return miniPhone(`${sb()}<div class="pb"><p class="ph-title" style="font-size:14px">api-server, Claude</p><div class="ph-card" style="flex:1"><p class="d">❯ Đã sửa xong login, chạy test pass.</p></div>${this.att ? `<div class="tag acc" style="align-self:flex-start">${I('cam', 11)} anh-loi.jpg → .orrery/inbox/</div>` : ''}<div class="ph-card" style="min-height:58px;font-size:12.5px">${rec ? '<div class="wave"><i></i><i></i><i></i><i></i><i></i><i></i></div>' : this.text ? esc(this.text) : '<span style="color:var(--m-muted)">Nhập hoặc giữ mic để nói…</span>'}</div><div style="display:flex;gap:8px"><button class="ph-btn sec" style="width:48px;flex:none" data-p4="cam" aria-label="Chụp ảnh">${I('cam', 18)}</button><button class="ph-btn ${rec ? '' : 'sec'}" style="flex:1;touch-action:none" data-p4="mic" aria-label="Giữ để nói">${I('mic', 18)} ${rec ? 'Thả ra' : 'Giữ để nói'}</button><button class="ph-btn" style="width:48px;flex:none" ${this.text ? '' : 'disabled'} aria-label="Gửi" data-p4="send">${I('send', 16)}</button></div></div>`)
    },
    acts: () => `<button class="chipbtn ghost" data-reset>${I('reset', 14)} Làm lại</button>`,
    on(e) {
      const b = e.target.closest('[data-p4]')
      if (!b) return
      if (b.dataset.p4 === 'cam') { this.att = true; if (!this.text) this.text = 'Xem ảnh lỗi trong .orrery/inbox/anh-loi.jpg, sửa giúp'; return true }
      if (b.dataset.p4 === 'send' && this.text) { this.text = ''; this.att = false; this.s = 'idle'; return true }
    },
  }
  D.P5 = {
    s: 'list',
    render() {
      const inner = this.s === 'list'
        ? `<p class="ph-title" style="font-size:15px">Cổng trên DESKTOP-JIN</p><button class="ph-card" data-p5 style="text-align:left;border:0;cursor:pointer;font:inherit;color:inherit"><p class="t">${I('globe', 12)} 5173, Vite</p><p class="d">web-app, npm run dev</p></button><div class="ph-card"><p class="t">8080, Gradle</p><p class="d">Không chia sẻ (chưa chọn)</p></div><p class="d" style="font-size:11.5px;color:var(--m-muted)">Chỉ hiện cổng chủ nhân đã bật chia sẻ trên máy tính.</p>`
        : `<div class="ph-card mono" style="padding:6px 10px">localhost:5173 qua DESKTOP-JIN</div><div class="tag ok" style="align-self:flex-start">${I('lock', 11)} Kênh mã hoá, tự đóng sau 29:58</div><div class="ph-card" style="flex:1;display:grid;place-items:center;text-align:center"><div><p class="t" style="font-size:16px">My Web App</p><p class="d">Trang dev đang chạy trên máy tính</p><button class="ph-btn" style="margin-top:10px">Đăng nhập</button></div></div><button class="ph-btn sec" data-p5back>Đóng</button>`
      return miniPhone(`${sb()}<div class="pb">${inner}</div>`)
    },
    acts: () => `<button class="chipbtn ghost" data-reset>${I('reset', 14)} Làm lại</button>`,
    on(e) { if (e.target.closest('[data-p5]')) { this.s = 'open'; return true } if (e.target.closest('[data-p5back]')) { this.s = 'list'; return true } },
  }
  D.P6 = {
    s: 'idle',
    render() {
      const head = `<div style="display:flex;align-items:center;gap:8px;flex-wrap:wrap"><b style="font-size:13px;flex:1">api-server, Claude</b><button class="b ${this.s === 'idle' ? 'pri' : ''}" data-p6 ${this.s !== 'idle' ? 'disabled' : ''}>${I('phone', 13)} ${this.s === 'idle' ? 'Mở trên điện thoại' : 'Đã gửi'}</button></div><div class="term"><span class="p">❯</span> refactor auth middleware<br>Reading src/auth.ts…</div>`
      let ph
      if (this.s === 'idle') ph = `${sb()}<div class="pb" style="background:var(--m-raised)">${appsGrid(true)}</div>`
      else if (this.s === 'sent') ph = `${sb()}<div class="pb" style="background:var(--m-raised)"><button class="notif" data-p6open style="border:0;text-align:left;font:inherit;color:inherit;cursor:pointer"><div class="h">${orrIcon} Orrery</div><p class="t">Tiếp tục api-server</p><p class="d">Chạm để mở đúng pane này</p></button>${appsGrid(true)}</div>`
      else ph = phApp(`<p class="t" style="font-weight:600;margin:0">api-server</p><div class="term"><span class="p">❯</span> refactor auth middleware<br>Reading src/auth.ts…</div><div class="ph-card"><p class="d">Gõ tiếp…</p></div>`)
      return miniWin('Workspace', head) + miniPhone(ph)
    },
    acts: () => `<button class="chipbtn ghost" data-reset>${I('reset', 14)} Làm lại</button>`,
    on(e) { if (e.target.closest('[data-p6]')) { this.s = 'sent'; return true } if (e.target.closest('[data-p6open]')) { this.s = 'open'; return true } },
  }
  D.D2 = {
    s: 0,
    render() {
      const screens = ['Màn đăng nhập', 'Trang chủ app', 'Cài đặt']
      return miniWin('Galaxy A33, mirror', `<div style="display:flex;gap:12px;align-items:flex-start;flex-wrap:wrap"><button class="mirror" data-d2 aria-label="Bấm vào màn điện thoại" style="border:0;cursor:pointer"><div class="scr2"><span>${screens[this.s % 3]}</span></div></button><div style="display:grid;gap:6px;min-width:0;flex:1 1 140px"><span class="tag ok" style="justify-self:start">${I('check', 11)} 30 fps</span><p class="d" style="margin:0;font-size:12.5px;color:var(--m-muted)">Chỉ stream khi pane hiện; thu nhỏ hoặc đổi tab là dừng.</p><div style="display:flex;gap:6px;flex-wrap:wrap"><button class="b" aria-label="Back">${I('back', 13)}</button><button class="b" aria-label="Home">${I('home', 13)}</button><button class="b" aria-label="Xoay">${I('rotate', 13)}</button><button class="b" data-toast="Đã gửi ảnh chụp cho agent">${I('cam', 13)} Gửi agent</button></div></div></div>`)
    },
    acts: () => '<span class="sub" style="margin:0;font-size:14px">Bấm vào màn điện thoại trong pane.</span>',
    on(e, el) {
      const b = e.target.closest('[data-d2]')
      if (!b) return
      const r = b.getBoundingClientRect()
      const dot = document.createElement('i')
      dot.className = 'tap'
      dot.style.left = `${e.clientX - r.left}px`
      dot.style.top = `${e.clientY - r.top}px`
      $('.scr2', b).appendChild(dot)
      setTimeout(() => { this.s++; paint('D2') }, 300)
      return false
    },
  }
  D.D3 = {
    s: 'idle', clip: 'https://github.com/…/pull/412',
    render() {
      const desk = `<div style="display:grid;gap:8px"><p class="d" style="margin:0;font-size:12.5px;color:var(--m-muted)">Clipboard máy tính</p><div class="mono" style="background:var(--m-raised);padding:6px 8px;border-radius:6px">${esc(this.clip)}</div><div style="display:flex;gap:6px;flex-wrap:wrap"><button class="b pri" data-d3="send">${I('send', 13)} Gửi sang điện thoại</button><button class="b" data-d3="secret">Thử với secret</button></div>${this.s === 'blocked' ? `<div class="tag err" role="alert">${I('alert', 11)} Giống API key, đã chặn không gửi</div>` : ''}</div>`
      const ph = this.s === 'sent'
        ? `${sb()}<div class="pb" style="background:var(--m-raised)"><div class="notif"><div class="h">${orrIcon} Orrery, từ DESKTOP-JIN</div><p class="t">Đã nhận link</p><p class="d mono">${esc(this.clip)}</p><div class="acts"><button class="pri">Copy</button><button>Mở</button></div></div>${appsGrid(true)}</div>`
        : `${sb()}<div class="pb" style="background:var(--m-raised)">${appsGrid(true)}</div>`
      return miniWin('Clipboard chung', desk) + miniPhone(ph)
    },
    acts: () => `<button class="chipbtn ghost" data-reset>${I('reset', 14)} Làm lại</button>`,
    on(e) {
      const b = e.target.closest('[data-d3]')
      if (!b) return
      if (b.dataset.d3 === 'secret') { this.clip = 'sk-proj-A1b2…'; this.s = 'idle'; return true }
      this.s = this.clip.startsWith('sk-') ? 'blocked' : 'sent'
      return true
    },
  }
  const WHY = {
    P1: 'Claude / Codex đang hỏi 1 / 2 / 3: bấm ngay trên thông báo. Chỉ gõ khi người bấm (ngoại lệ 6, 7); màn khoá bắt mở khoá trước.',
    P2: 'Liếc màn hình chính là biết có cần mình không. Widget chỉ cập nhật khi có push, không tự hỏi máy chủ.',
    P3: 'Agent chạy lâu: 1 thông báo cố định cho mỗi session, đổi thành "Xong" kèm nút Mở.',
    P4: 'Giữ mic để đọc prompt, thả ra thì chữ hiện để sửa, bấm Gửi mới gửi. Ảnh lưu vào project, chèn đường dẫn.',
    P5: 'Mở dev server của máy tính trên điện thoại khi không cùng Wi-Fi. Tắt mặc định, chỉ cổng được chọn, tự đóng 30 phút.',
    P6: 'Đang đứng dậy rời bàn: 1 nút gửi pane sang điện thoại, chạm thông báo là vào đúng pane.',
    D2: 'Màn Android ngay trong Workspace, bấm được. Không chạy nền khi pane ẩn để giữ Workspace mượt.',
    D3: 'Gửi link, đoạn text giữa máy tính và điện thoại khi bấm. Nội dung giống secret thì chặn.',
  }
  const later = (id, ms, fn) => setTimeout(() => { fn(); paint(id) }, RM.matches ? 0 : ms)
  const runLog = '<span class="p">❯</span> ./gradlew assembleDebug<br>&gt; Task :app:compileDebugKotlin<br>&gt; Task :app:packageDebug<br><span style="color:#4cc38a">BUILD SUCCESSFUL in 41s</span><br>app-debug.apk, 18.2 MB'
  D.E1 = {
    s: 'idle', p: 0,
    render() {
      const sent = this.s !== 'idle'
      const desk = miniWin('web-app, Run Android', `<div class="term">${runLog}</div><div style="display:flex;gap:8px;align-items:center;flex-wrap:wrap;margin-top:8px"><button class="b ${sent ? '' : 'pri'}" data-e1="send" ${sent ? 'disabled' : ''}>${I(sent ? 'check' : 'send', 13)} ${sent ? 'Đã gửi tới Galaxy A33' : 'Gửi lên điện thoại'}</button><span class="m-sub" style="margin:0;display:flex;align-items:center;gap:6px">Tự gửi khi build xong <button class="sw" role="switch" aria-checked="false" aria-label="Tự gửi khi build xong" data-sw></button></span></div>`)
      let ph = ''
      if (this.s === 'idle') ph = `<div class="pb" style="background:var(--m-raised)">${appsGrid(true)}</div>`
      else if (this.s === 'sent') ph = `<div class="pb" style="background:var(--m-raised)"><div class="notif"><div class="h">${orrIcon} Orrery, web-app</div><p class="t">app-debug.apk build xong</p><p class="d">18.2 MB, bản debug, ký giống bản đang cài</p><div class="acts"><button class="pri" data-e1="dl">${I('dl', 12)} Cài</button><button>Để sau</button></div></div>${appsGrid(true)}</div>`
      else if (this.s === 'dl') ph = phApp(`<div class="ph-card"><p class="t">Đang tải app-debug.apk</p><p class="d">Qua Wi-Fi nội bộ, ${Math.round(this.p)}%</p><div class="prog"><i style="--p:${this.p / 100}"></i></div></div>`)
      else if (this.s === 'confirm') ph = `<div class="pb" style="background:var(--m-raised)"><div class="sysd"><p class="t" style="font-weight:700;margin:0 0 4px">web-app</p><p class="d" style="font-size:12px;color:var(--m-muted);margin:0 0 12px">Bạn có muốn cài bản cập nhật cho app này?</p><div style="display:flex;gap:8px"><button class="ph-btn sec" data-e1="cancel">Huỷ</button><button class="ph-btn" data-e1="inst">Cài</button></div></div></div>`
      else if (this.s === 'inst') ph = `<div class="pb" style="background:var(--m-raised)"><div class="sysd"><p class="t" style="font-weight:700;margin:0 0 8px">Đang cài…</p><div class="prog ind"><i></i></div></div></div>`
      else ph = `<div class="pb" style="background:var(--m-raised)"><div class="sysd"><p class="t" style="font-weight:700;margin:0 0 12px">${I('check', 14)} Đã cài web-app</p><div style="display:flex;gap:8px"><button class="ph-btn sec" data-e1="cancel">Xong</button><button class="ph-btn">Mở</button></div></div></div>`
      return desk + miniPhone(sb() + ph)
    },
    acts: () => `<button class="chipbtn ghost" data-reset>${I('reset', 14)} Làm lại</button>`,
    on(e) {
      const a = e.target.closest('[data-e1]')?.dataset.e1
      if (!a) return
      if (a === 'send') this.s = 'sent'
      if (a === 'cancel') this.s = 'idle'
      if (a === 'dl') {
        this.s = 'dl'; this.p = 0
        const tick = () => { if (this.s !== 'dl') return; this.p = Math.min(100, this.p + 14 + Math.random() * 12); if (this.p >= 100) this.s = 'confirm'; paint('E1'); if (this.s === 'dl') setTimeout(tick, 220) }
        setTimeout(tick, 200)
      }
      if (a === 'inst') { this.s = 'inst'; later('E1', 1300, () => { this.s = 'done' }) }
      return true
    },
  }
  const crashLines = '<span style="color:#ff6b6b">FATAL EXCEPTION: main</span><br><span style="color:#ff6b6b">java.lang.NullPointerException: user is null</span><br><span style="color:#ff6b6b">  at LoginViewModel.submit(LoginViewModel.kt:42)</span><br><span style="color:#ff6b6b">  at LoginScreen$1.invoke(LoginScreen.kt:88)</span>'
  D.E2 = {
    s: 'run',
    render() {
      const log = `I/web-app: MainActivity onCreate<br>D/Net: GET /me 200 84ms<br>I/web-app: LoginScreen shown${this.s !== 'run' ? '<br>' + crashLines : '<br><span style="color:#8e8e9a">…</span>'}`
      let body = `<div style="display:flex;align-items:center;gap:6px;flex-wrap:wrap"><b style="font-size:13px">web-app, Run</b><span class="tag ${this.s === 'run' ? 'ok' : 'err'}">${this.s === 'run' ? I('play', 10) + ' Đang chạy' : I('alert', 10) + ' Crash'}</span></div><div class="term" style="font-size:10.5px;flex:1">${log}</div>`
      if (this.s === 'crash') body += `<button class="ph-btn" data-e2="pick">${I('send', 15)} Gửi crash cho agent</button>`
      if (this.s === 'pick') body += `<div class="sysd" style="margin-top:0"><p class="t" style="font-weight:700;margin:0 0 8px">Gửi vào pane nào?</p><button class="ph-btn sec" data-e2="a" style="margin-bottom:6px">api-server, Claude</button><button class="ph-btn sec" data-e2="b">web-app, Codex</button></div>`
      if (this.s === 'sent') body += `<div class="tag ok" style="align-self:flex-start">${I('check', 11)} Đã gửi vào web-app, Codex</div>`
      const desk = miniWin('web-app, Codex', `<div class="term">${this.s === 'sent' ? '<span class="p">❯</span> App crash khi bấm Đăng nhập, sửa giúp:<br>NullPointerException: user is null<br>at LoginViewModel.kt:42 …' : '<span class="p">❯</span> <span style="color:#8e8e9a">chờ prompt</span>'}</div>`)
      return miniPhone(`${sb()}<div class="pb">${body}</div>`) + desk
    },
    acts() { return `<button class="chipbtn ghost" data-e2="crash" ${this.s !== 'run' ? 'disabled' : ''}>${I('alert', 14)} Mô phỏng crash</button><button class="chipbtn ghost" data-reset>${I('reset', 14)} Làm lại</button>` },
    on(e) {
      const a = e.target.closest('[data-e2]')?.dataset.e2
      if (!a) return
      this.s = a === 'crash' ? 'crash' : a === 'pick' ? 'pick' : 'sent'
      return true
    },
  }
  const DEVS3 = [['a33', 'Galaxy A33', '9/19.5'], ['px8', 'Pixel 8', '9/20'], ['tab', 'Tablet API 35', '4/3']]
  D.E3 = {
    s: 'pick', sel: { a33: true, px8: true, tab: true }, v: 'light',
    shot(id, ratio) {
      const dark = this.v === 'dark', big = this.v === 'font'
      const bg = dark ? '#1b1b1f' : '#fff', ink = dark ? '#ececf1' : '#17171c'
      const small = id === 'a33' && big
      return `<div style="aspect-ratio:${ratio};background:${bg};color:${ink};border-radius:8px;box-shadow:0 0 0 1px var(--m-line-2);padding:8px;display:flex;flex-direction:column;gap:5px;overflow:hidden;font-size:${big ? 12 : 9.5}px;line-height:1.3"><b style="font-size:${big ? 14 : 11}px">Đăng nhập</b><span style="height:14px;border-radius:4px;background:${dark ? '#2a2a30' : '#ececf1'}"></span><span style="height:14px;border-radius:4px;background:${dark ? '#2a2a30' : '#ececf1'}"></span><span style="margin-top:auto;background:#bd5128;color:#fff;border-radius:6px;padding:4px 6px;white-space:nowrap;overflow:hidden;${small ? 'width:70%' : ''}">Tiếp tục với tài khoản Google</span></div>`
    },
    render() {
      const chips = DEVS3.map(([id, n]) => `<button class="b ${this.sel[id] ? 'pri' : ''}" data-e3d="${id}" aria-pressed="${!!this.sel[id]}">${I(this.sel[id] ? 'check' : 'plus', 12)} ${n}</button>`).join('')
      const seg = `<div class="seg-m" role="group" aria-label="Biến thể">${[['light', 'Sáng'], ['dark', 'Tối'], ['font', 'Chữ 1.3x']].map(([k, n]) => `<button class="${this.v === k ? 'on' : ''}" data-e3v="${k}">${n}</button>`).join('')}</div>`
      const picked = DEVS3.filter(([id]) => this.sel[id])
      let out = ''
      if (this.s === 'run') out = `<div class="panel"><ul class="chk">${picked.map(([, n]) => `<li><span class="ic acc">${I('spin', 13)}</span><span class="grow">${n}</span><span class="v">cài, mở, chụp</span></li>`).join('')}</ul></div>`
      if (this.s === 'grid') out = `<div style="display:grid;grid-template-columns:repeat(${picked.length},minmax(0,1fr));gap:10px;align-items:end">${picked.map(([id, n, r]) => `<figure style="margin:0;display:grid;gap:4px">${this.shot(id, r)}<figcaption style="font-size:11.5px;color:var(--m-muted);display:flex;justify-content:space-between;gap:4px;align-items:center;flex-wrap:wrap"><span>${n}</span><button class="b link" style="min-height:24px" data-toast="Đã gửi ảnh ${n} cho agent">Gửi agent</button></figcaption></figure>`).join('')}</div>`
      return miniWin('web-app, Run trên nhiều máy', `<div style="display:flex;flex-wrap:wrap;gap:6px">${chips}</div><div style="display:flex;gap:8px;align-items:center;flex-wrap:wrap">${seg}<button class="b pri" data-e3="run" ${picked.length && this.s !== 'run' ? '' : 'disabled'}>${I('play', 13)} Chạy và chụp</button></div>${out}<p class="m-sub" style="margin:0">Chạy song song max 3 máy. Biến thể đổi qua adb (dark mode, font scale), xong trả lại như cũ.</p>`)
    },
    acts: () => `<button class="chipbtn ghost" data-reset>${I('reset', 14)} Làm lại</button>`,
    on(e) {
      const d = e.target.closest('[data-e3d]')
      if (d) { this.sel = { ...this.sel, [d.dataset.e3d]: !this.sel[d.dataset.e3d] }; return true }
      const v = e.target.closest('[data-e3v]')
      if (v) { this.v = v.dataset.e3v; if (this.s === 'grid') { this.s = 'run'; later('E3', 1400, () => { this.s = 'grid' }) } return true }
      if (e.target.closest('[data-e3="run"]')) { this.s = 'run'; later('E3', 1600, () => { this.s = 'grid' }); return true }
    },
  }
  D.E4 = {
    s: 'closed',
    render() {
      const tile = (icon, name, sub, on, attr = '') => `<button ${attr} style="border:0;cursor:pointer;display:flex;align-items:center;gap:8px;padding:10px;border-radius:18px;background:${on === 'acc' ? 'var(--m-accent)' : on ? 'var(--m-sel)' : 'var(--m-raised)'};color:${on === 'acc' ? 'var(--m-on-accent)' : 'var(--m-ink)'};font:600 11.5px/1.2 var(--font);text-align:left;min-height:52px;min-width:0"><span style="flex:none">${I(icon, 16)}</span><span style="min-width:0"><span style="display:block">${name}</span><span style="display:block;font-weight:500;opacity:.8">${sub}</span></span></button>`
      let ph
      if (this.s === 'closed') ph = `<div class="pb" style="background:var(--m-raised)"><button class="ph-btn sec" data-e4="open">${I('chev', 14, '')} Vuốt xuống từ mép trên</button>${appsGrid(true)}</div>`
      else if (this.s === 'app') ph = phApp(sessionsList)
      else ph = `<div class="pb" style="background:#0b0b0e"><div style="display:grid;grid-template-columns:1fr 1fr;gap:8px;animation:drop var(--t-med) var(--ease-out)">${tile('wifi', 'Wi-Fi', 'Nhà', true)}${tile('zap', 'Đèn pin', 'Tắt', false)}${tile('bell', 'Không làm phiền', 'Tắt', false)}${this.s === 'paused' ? tile('moon', 'Orrery', 'Tạm dừng, 59:59', false, 'data-e4="resume"') : tile('globe', 'Orrery', '2 chờ bạn', 'acc', 'data-e4="app"')}</div><p style="color:#bbb;font-size:11.5px;text-align:center;margin:6px 0 0">${this.s === 'paused' ? 'Chạm để bật lại thông báo' : 'Chạm mở app, giữ để tạm dừng 1 giờ'}</p></div>`
      return miniPhone(sb() + ph)
    },
    acts() { return `<button class="chipbtn ghost" data-e4="hold" ${this.s === 'open' ? '' : 'disabled'}>${I('moon', 14)} Mô phỏng: giữ ô Orrery</button><button class="chipbtn ghost" data-reset>${I('reset', 14)} Làm lại</button>` },
    on(e) {
      const a = e.target.closest('[data-e4]')?.dataset.e4
      if (!a) return
      this.s = { open: 'open', app: 'app', hold: 'paused', resume: 'open' }[a]
      return true
    },
  }
  D.E5 = {
    s: 'type', cmd: 'git push --force origin main',
    render() {
      const danger = /--force|reset --hard|rm -rf/.test(this.cmd)
      let extra = ''
      if (this.s === 'auth') extra = `<div class="sheet-wrap" style="border-radius:29px"><div class="sheet" style="text-align:center"><span class="ic warn" style="width:44px;height:44px;margin:0 auto 8px">${I('alert', 22)}</span><h4>Lệnh nguy hiểm</h4><p>Ghi đè lịch sử nhánh main trên remote. Chạm vân tay để gửi.</p><button class="ph-btn" data-e5="ok" style="margin-bottom:8px">${I('lock', 16)} Chạm cảm biến vân tay</button><button class="ph-btn sec" data-e5="no">Huỷ</button></div></div>`
      const out = this.s === 'sent' ? `<br><span class="p">❯</span> ${esc(this.cmd)}<br>${danger ? '+ 3f2a1c9...8d0e4b2 main -> main (forced update)' : 'On branch feat/login, nothing to commit'}` : this.s === 'cancel' ? '<br><span style="color:#8e8e9a">Đã huỷ, lệnh không được gửi</span>' : ''
      return miniPhone(`${sb()}<div class="pb" style="position:relative"><p class="ph-title" style="font-size:14px">web-app, shell</p><div class="term" style="flex:1;font-size:10.5px"><span class="p">❯</span> git log --oneline -1<br>8d0e4b2 fix login${out}</div><div class="ph-card mono" style="font-size:11.5px">${esc(this.cmd)}</div><button class="ph-btn" data-e5="send" ${this.s === 'type' ? '' : 'disabled'}>${I('send', 15)} Gửi</button>${extra}</div>`)
    },
    acts() { return `<button class="chipbtn ghost" data-e5="safe">${I('term', 14)} Thử lệnh an toàn</button><button class="chipbtn ghost" data-reset>${I('reset', 14)} Làm lại</button>` },
    on(e) {
      const a = e.target.closest('[data-e5]')?.dataset.e5
      if (!a) return
      if (a === 'safe') { this.cmd = 'git status'; this.s = 'type' }
      if (a === 'send') this.s = /--force|reset --hard|rm -rf/.test(this.cmd) ? 'auth' : 'sent'
      if (a === 'ok') this.s = 'sent'
      if (a === 'no') this.s = 'cancel'
      return true
    },
  }
  D.E6 = {
    s: 'idle', ask: false, items: null,
    list() {
      if (!this.items) this.items = [
        { id: 'tc', t: 'typecheck', st: 'ok', d: 'npm run typecheck, 18s' },
        { id: 'ts', t: 'test', st: 'fail', d: '2 test lỗi trong auth.test.ts' },
        { id: 'cl', t: 'Cập nhật CHANGELOG', todo: true, done: false, d: 'Claude thêm qua todo_add' },
        { id: 'lg', t: 'Xoá log debug', todo: true, done: true, d: 'Claude thêm qua todo_add' },
      ]
      return this.items
    },
    render() {
      const L = this.list()
      const ok = L.filter((x) => (x.todo ? x.done : x.st === 'ok')).length
      const rows = L.map((x) => {
        const good = x.todo ? x.done : x.st === 'ok'
        const run = x.st === 'run'
        const ic = run ? `<span class="ic acc" style="width:26px;height:26px">${I('spin', 13)}</span>` : x.todo ? `<button class="ic ${good ? 'ok' : ''}" data-e6t="${x.id}" aria-pressed="${good}" aria-label="Đánh dấu ${esc(x.t)}" style="width:26px;height:26px;border:0;cursor:pointer">${good ? I('check', 13) : ''}</button>` : `<span class="ic ${good ? 'ok' : 'err'}" style="width:26px;height:26px">${I(good ? 'check' : 'x', 13)}</span>`
        return `<div class="ph-card" style="display:flex;gap:8px;align-items:center;padding:8px 10px">${ic}<div style="min-width:0"><p class="t" style="${x.todo && good ? 'text-decoration:line-through;color:var(--m-muted)' : ''}">${esc(x.t)}</p><p class="d">${esc(x.d)}</p></div></div>`
      }).join('')
      const all = ok === L.length
      const ask = this.ask ? `<div class="sheet-wrap" style="border-radius:29px"><div class="sheet"><h4>Chạy lại kiểm trên DESKTOP-JIN?</h4><p>Khoảng 1 phút, dùng CPU máy nhà.</p><div style="display:flex;gap:8px"><button class="ph-btn sec" data-e6="no">Huỷ</button><button class="ph-btn" data-e6="go">Chạy</button></div></div></div>` : ''
      return miniPhone(`${sb()}<div class="pb" style="position:relative;gap:6px"><p class="ph-title" style="font-size:14px">Trước khi merge feat/login</p><div style="display:flex;align-items:center;gap:8px"><span class="tag ${all ? 'ok' : 'wait'}">${all ? I('check', 11) + ' Sẵn sàng merge' : `${ok}/${L.length} xong`}</span></div>${rows}${ask}${`<button class="ph-btn ${all ? '' : 'sec'}" data-e6="ask" ${L.some((x) => x.st === 'run') ? 'disabled' : ''}>${I('reset', 15)} Chạy lại kiểm</button>`}</div>`)
    },
    acts: () => `<button class="chipbtn ghost" data-reset>${I('reset', 14)} Làm lại</button>`,
    on(e) {
      const t = e.target.closest('[data-e6t]')
      if (t) { const x = this.list().find((i) => i.id === t.dataset.e6t); x.done = !x.done; return true }
      const a = e.target.closest('[data-e6]')?.dataset.e6
      if (a === 'ask') { this.ask = true; return true }
      if (a === 'no') { this.ask = false; return true }
      if (a === 'go') {
        this.ask = false
        const L = this.list()
        L.filter((x) => !x.todo).forEach((x) => { x.st = 'run' })
        later('E6', 1700, () => { L.filter((x) => !x.todo).forEach((x) => { x.st = 'ok'; if (x.id === 'ts') x.d = '48 test pass, 22s' }) })
        return true
      }
    },
  }
  WHY.E1 = 'Build xong trên máy tính, điện thoại nhận thông báo "Cài". Android vẫn hỏi xác nhận cài như bình thường; chỉ nhận APK từ thư mục build của project.'
  WHY.E2 = 'Đang ở ngoài vẫn thấy log Run và crash. Gửi stack trace vào pane chọn, chỉ khi chủ nhân bấm.'
  WHY.E3 = 'Chạy 1 lần trên 2-3 máy, lưới ảnh chụp cùng lúc. Bấm biến thể Tối / Chữ 1.3x để thấy nút bị cắt trên máy nhỏ.'
  WHY.E4 = 'Ô Orrery trong Quick Settings: chạm mở app, giữ để tạm dừng thông báo 1 giờ.'
  WHY.E5 = 'Lệnh khớp mẫu nguy hiểm phải chạm vân tay trước khi gửi; lệnh thường gửi luôn. Bấm "Thử lệnh an toàn" để so.'
  WHY.E6 = 'Tick todo agent đã thêm, xem typecheck / test, chạy lại kiểm từ xa (hỏi trước vì dùng CPU máy nhà).'

  const GQ = [
    ['api-server', 'Claude hỏi: sửa auth.ts?', 'Do you want to make this edit to auth.ts?', ['1 Yes', '2 Allow all', '3 No']],
    ['web-app', 'Codex chờ chọn cách merge', 'Chọn cách gộp nhánh feat/login vào main', ['1 Squash', '2 Rebase', '3 Merge']],
    ['docs', 'Claude hỏi: chạy npm test?', 'Run npm test in docs/ ?', ['1 Yes', '2 No']],
  ]
  D.G1 = {
    i: 0, res: [], last: null,
    render() {
      const n = GQ.length
      const dots = GQ.map((_, k) => `<i style="width:${k === this.i ? 18 : 6}px;height:6px;border-radius:3px;background:${k < this.i ? 'var(--m-done)' : k === this.i ? 'var(--m-accent)' : 'var(--m-line-2)'};transition:width var(--t-med) var(--ease-out)"></i>`).join('')
      let body
      if (this.i >= n) {
        body = `<div class="empty-m swap"><span class="ic ok" style="width:44px;height:44px">${I('check', 22)}</span><p class="t" style="font-weight:700;margin:0">Hết việc chờ bạn</p><p class="d" style="margin:0;color:var(--m-muted);font-size:12px">${this.res.filter((r) => r[1] === 'snooze').length} session được hoãn, sẽ nhắc lại sau 15 phút.</p></div>`
      } else {
        const [name, ttl, q, ans] = GQ[this.i]
        body = `<div class="ph-card swap"><div class="tags" style="margin:0 0 6px"><span class="tag wait">${I('clock', 11)} ${name}</span></div><p class="t">${esc(ttl)}</p><p class="d">${esc(q)}</p></div>${ans.map((a, k) => `<button class="ph-btn ${k ? 'sec' : ''}" data-g1="a${k}">${esc(a)}</button>`).join('')}<div style="display:flex;gap:8px"><button class="ph-btn sec" data-g1="snooze" style="min-width:0;font-size:12.5px;white-space:nowrap;padding:0 4px">Hoãn 15 phút</button><button class="ph-btn sec" data-g1="skip" style="min-width:0;font-size:12.5px;white-space:nowrap;padding:0 4px">Bỏ qua</button></div>`
      }
      const undo = this.last ? `<div class="tag ok" style="align-self:stretch;justify-content:space-between;gap:8px;border-radius:12px;white-space:normal">${esc(this.last)} <button class="b link" data-g1="undo" style="min-height:44px">${I('undo', 12)} Hoàn tác</button></div>` : ''
      const ph = `${sb()}<div class="pb" style="gap:8px"><div style="display:flex;align-items:center;justify-content:space-between"><b style="font-size:14px">Chờ bạn ${Math.min(this.i + 1, n)}/${n}</b><span style="display:flex;gap:4px;align-items:center">${dots}</span></div>${body}${undo}</div>`
      const rows = GQ.map(([name], k) => {
        const r = this.res.find((x) => x[0] === k)
        const st = r ? (r[1] === 'ans' ? ['ok', 'Đã trả lời'] : r[1] === 'snooze' ? ['', 'Hoãn 15 phút'] : ['', 'Bỏ qua']) : ['wait', 'Chờ bạn']
        return `<div class="row"><span class="ic ${st[0] || ''}">${I(st[0] === 'ok' ? 'check' : st[0] === 'wait' ? 'bell' : 'clock', 14)}</span><div class="grow"><p class="t">${name}</p></div><span class="tag ${st[0]}">${st[1]}</span></div>`
      }).join('')
      return miniPhone(ph) + miniWin('Desktop, Inbox', `<div class="panel">${rows}</div><p class="m-sub" style="margin:0">Desktop cập nhật theo điện thoại ngay khi bạn bấm.</p>`)
    },
    acts: () => `<button class="chipbtn ghost" data-reset>${I('reset', 14)} Làm lại</button>`,
    on(e) {
      const a = e.target.closest('[data-g1]')?.dataset.g1
      if (!a) return
      if (a === 'undo') { this.res.pop(); this.i = Math.max(0, this.i - 1); this.last = null; return true }
      const name = GQ[this.i][0]
      if (a.startsWith('a')) { this.res.push([this.i, 'ans']); this.last = `Đã gõ "${GQ[this.i][3][+a.slice(1)].split(' ')[0]}" vào ${name}` }
      if (a === 'snooze') { this.res.push([this.i, 'snooze']); this.last = `Đã hoãn ${name}` }
      if (a === 'skip') { this.res.push([this.i, 'skip']); this.last = `Đã bỏ qua ${name}` }
      this.i++
      return true
    },
  }

  const GP = ['Chạy test rồi báo lại kết quả', 'Review thay đổi trên {branch}', 'Giải thích file {file}']
  const GFILL = (t) => t.replace('{branch}', 'feat/login').replace('{file}', 'auth.ts')
  D.G2 = {
    s: 'idle', txt: '',
    render() {
      const list = GP.map((p, k) => `<button class="b" data-g2="${k}" style="justify-content:flex-start;min-height:44px;width:100%;white-space:normal;text-align:left">${I('send', 13)} ${esc(p)}</button>`).join('')
      const pal = `<div class="panel" style="padding:8px;display:grid;gap:6px"><div class="mono" style="background:var(--m-raised);padding:8px;border-radius:6px">; prompt đã lưu</div>${list}</div>`
      const out = this.s === 'sent' ? `<br><span style="color:#4cc38a">Claude nhận prompt, đang chạy…</span>` : ''
      const caret = this.s === 'ins' ? '<span style="opacity:.8;animation:wv 1s steps(2) infinite">▌</span>' : ''
      const term = `<div class="term"><span class="p">❯</span> ${esc(this.txt)}${caret}${out}</div>`
      const hint = this.s === 'ins' ? `<div class="tag wait" style="align-self:flex-start">${I('alert', 11)} Chưa gửi, bấm Enter trong pane để gửi</div><button class="b pri" data-g2="enter" style="min-height:44px">Mô phỏng: bạn bấm Enter</button>` : this.s === 'sent' ? `<div class="tag ok" style="align-self:flex-start">${I('check', 11)} Đã gửi do bạn bấm Enter</div>` : `<p class="m-sub" style="margin:0">Chọn một prompt để chèn vào ô nhập.</p>`
      const chips = GP.map((p, k) => `<button class="b" data-g2="${k}" style="min-height:44px;white-space:normal;text-align:left;font-size:12px">${esc(p.split(' ').slice(0, 3).join(' '))}…</button>`).join('')
      const ph = `${sb()}<div class="pb"><p class="ph-title" style="font-size:14px">api-server, Claude</p><div class="term" style="flex:1;font-size:10.5px">Claude: xong, chờ bạn.</div><div style="display:grid;gap:6px">${chips}</div><div class="ph-card mono" style="font-size:11.5px;min-height:40px">${esc(this.txt) || '<span style="opacity:.6">Nhập tin nhắn</span>'}</div></div>`
      return miniWin('api-server, Claude, Ctrl+K', `${pal}${term}${hint}`) + miniPhone(ph)
    },
    acts: () => `<button class="chipbtn ghost" data-reset>${I('reset', 14)} Làm lại</button>`,
    on(e) {
      const a = e.target.closest('[data-g2]')?.dataset.g2
      if (a === undefined) return
      if (a === 'enter') { this.s = 'sent'; return true }
      this.txt = GFILL(GP[+a]); this.s = 'ins'
      return true
    },
  }

  const G3N = [['api-server', 'Edit auth.ts', 'wait'], ['web-app', 'Chạy test', 'run'], ['docs', 'Xong', 'done'], ['mobile', 'Build APK', 'run'], ['infra', 'Chờ chọn 1/2/3', 'wait'], ['cli', 'Xong', 'done'], ['site', 'Deploy', 'run'], ['relay', 'Lỗi build', 'err'], ['tools', 'Xong', 'done'], ['lab', 'Đọc code', 'run']]
  const G3H = { cozy: 28, normal: 38, roomy: 50 }
  const G3C = { wait: 'var(--m-wait)', run: 'var(--m-accent)', done: 'var(--m-done)', err: 'var(--m-err)' }
  D.G3 = {
    d: 'normal',
    render() {
      const seg = `<div class="seg-m" role="group" aria-label="Mật độ">${[['cozy', 'Gọn'], ['normal', 'Vừa'], ['roomy', 'Thoáng']].map(([k, n]) => `<button class="${this.d === k ? 'on' : ''}" data-g3="${k}" aria-pressed="${this.d === k}" style="min-height:44px">${n}</button>`).join('')}</div>`
      const rows = G3N.map(([n, l, s]) => `<div class="g3row" style="height:var(--rh);display:flex;align-items:center;gap:8px;padding:0 10px;border-top:1px solid var(--m-line);transition:height var(--t-med) var(--ease-out)"><i class="dot" style="background:${G3C[s]}"></i><b style="font-size:13px">${n}</b><span style="font-size:12px;color:var(--m-muted);white-space:nowrap;overflow:hidden;text-overflow:ellipsis;min-width:0">${l}</span></div>`).join('')
      return miniWin('Rail, 10 session', `${seg}<div class="panel" data-g3box style="--rh:${G3H[this.d]}px;height:228px;overflow:hidden">${rows}</div><p class="m-sub" style="margin:0" data-g3n>Khung 228 px nhìn thấy ${Math.floor(228 / G3H[this.d])} trên 10 dòng.</p>`)
    },
    acts: () => `<button class="chipbtn ghost" data-reset>${I('reset', 14)} Làm lại</button>`,
    on(e, el) {
      const b = e.target.closest('[data-g3]')
      if (!b) return
      this.d = b.dataset.g3
      const box = $('[data-g3box]', el)
      box.style.setProperty('--rh', `${G3H[this.d]}px`)
      $$('[data-g3]', el).forEach((x) => { x.classList.toggle('on', x === b); x.setAttribute('aria-pressed', String(x === b)) })
      $('[data-g3n]', el).textContent = `Khung 228 px nhìn thấy ${Math.floor(228 / G3H[this.d])} trên 10 dòng.`
      return false
    },
  }

  const G4S = [
    ['api-server', 'Claude', 'wait', ['Edit src/auth.ts', 'Do you want to make this edit?', '1. Yes  2. Allow all  3. No']],
    ['web-app', 'Codex', 'run', ['> npm test -- auth', '12 passed, 2 failed', 'Đang sửa auth.test.ts']],
    ['docs', 'Claude', 'done', ['Cập nhật README.md', '12 file đổi', 'Xong sau 6:40']],
  ]
  D.G4 = {
    open: '',
    render() {
      const rows = G4S.map(([n, a, s, ln]) => {
        const on = this.open === n
        const pop = on ? `<div class="term swap" role="tooltip" style="margin:0 0 6px;font-size:11px;min-height:0"><span style="color:#8e8e9a">3 dòng cuối, tool: ${n === 'web-app' ? 'Bash' : 'Edit'}</span><br>${ln.map(esc).join('<br>')}</div>` : ''
        return `<div class="row" style="padding:0"><button class="b" data-g4="${n}" aria-expanded="${on}" style="width:100%;justify-content:flex-start;gap:8px;min-height:44px;border-radius:0;background:${on ? 'var(--m-sel)' : 'transparent'}"><i class="dot" style="background:${G3C[s]}"></i><b>${n}</b><span style="color:var(--m-muted);font-weight:500">${a}</span></button></div>${pop ? `<div style="padding:6px 10px 0;border-top:1px solid var(--m-line)">${pop}</div>` : ''}`
      }).join('')
      return miniWin('Rail, xem nhanh', `<div class="panel">${rows}</div><p class="m-sub" style="margin:0">Trên desktop: rê chuột 0,4 giây. Trên phone: chạm giữ. Demo này: bấm một dòng, Esc để đóng. Tab đang xem không đổi.</p>`)
    },
    acts: () => `<button class="chipbtn ghost" data-reset>${I('reset', 14)} Làm lại</button>`,
    on(e) {
      const b = e.target.closest('[data-g4]')
      if (!b) return
      this.open = this.open === b.dataset.g4 ? '' : b.dataset.g4
      return true
    },
  }

  const G5L = [['api-server', 'wait', 12], ['web-app', 'done', 30], ['docs', 'done', 44], ['infra', 'err', 90], ['mobile', 'done', 120], ['site', 'wait', 150], ['cli', 'done', 300]]
  const G5T = { wait: ['wait', 'Chờ bạn'], done: ['ok', 'Xong'], err: ['err', 'Lỗi'] }
  D.G5 = {
    away: 45, f: '', seen: false,
    render() {
      const seg = `<div class="seg-m" role="group" aria-label="Thời gian vắng">${[[5, '5 phút'], [45, '45 phút'], [180, '3 giờ']].map(([m, n]) => `<button class="${this.away === m ? 'on' : ''}" data-g5a="${m}" style="min-height:44px" aria-pressed="${this.away === m}">${n}</button>`).join('')}</div>`
      const chg = G5L.filter((x) => x[2] <= this.away)
      const cnt = (k) => chg.filter((x) => x[1] === k).length
      let card = ''
      if (this.away < 30) card = `<div class="tag" style="align-self:flex-start">Vắng dưới 30 phút: không hiện thẻ</div>`
      else if (!this.seen) card = `<div class="ph-card swap"><p class="t">Từ lúc bạn đi vắng ${this.away === 45 ? '45 phút' : '3 giờ'}</p><div style="display:flex;gap:6px;flex-wrap:wrap;margin-top:8px">${[['done', 'xong'], ['wait', 'chờ bạn'], ['err', 'lỗi']].map(([k, n]) => `<button class="b ${this.f === k ? 'pri' : ''}" data-g5f="${k}" aria-pressed="${this.f === k}" style="min-height:44px" ${cnt(k) ? '' : 'disabled'}>${cnt(k)} ${n}</button>`).join('')}</div><button class="b link" data-g5f="seen" style="min-height:44px;margin-top:4px">Đã xem</button></div>`
      const list = G5L.filter((x) => !this.f || (x[1] === this.f && x[2] <= this.away)).map(([n, s, m]) => `<div class="ph-card" style="display:flex;gap:8px;align-items:center;padding:8px 10px"><div class="grow" style="flex:1;min-width:0"><p class="t">${n}</p><p class="d">${m < 60 ? m + ' phút' : Math.round(m / 60) + ' giờ'}</p></div><span class="tag ${G5T[s][0]}">${G5T[s][1]}</span></div>`).join('')
      return `<div style="display:grid;gap:8px;min-width:0;flex:1 1 200px"><p class="m-sub" style="margin:0">Mô phỏng: bạn rời điện thoại trong</p>${seg}</div>` + miniPhone(`${sb()}<div class="pb" style="gap:8px;overflow:auto"><p class="ph-title" style="font-size:15px;margin:0">Session</p>${card}${list}</div>`)
    },
    acts: () => `<button class="chipbtn ghost" data-reset>${I('reset', 14)} Làm lại</button>`,
    on(e) {
      const m = e.target.closest('[data-g5a]')
      if (m) { this.away = +m.dataset.g5a; this.f = ''; this.seen = false; return true }
      const f = e.target.closest('[data-g5f]')?.dataset.g5f
      if (!f) return
      if (f === 'seen') { this.seen = true; this.f = ''; return true }
      this.f = this.f === f ? '' : f
      return true
    },
  }
  WHY.G1 = 'Mỗi lần một thẻ: trả lời bằng nút menu, Hoãn 15 phút hoặc Bỏ qua; xong tự sang thẻ kế. Bên phải là Inbox desktop cập nhật theo. Có Hoàn tác.'
  WHY.G2 = 'Chọn prompt đã lưu (bên desktop qua Ctrl+K, bên phone qua chip): chữ được chèn vào ô nhập, chưa gửi. Chỉ khi bạn bấm Enter mới gửi.'
  WHY.G3 = 'Bấm Gọn, Vừa, Thoáng: khoảng cách hàng đổi mượt, số session nhìn thấy trong cùng khung đổi theo. Mặc định Vừa như hiện tại.'
  WHY.G4 = 'Xem 3 dòng cuối và tool đang chạy của session mà không rời tab hiện tại. Chỉ vẽ khi rê chuột, không chạy nền.'
  WHY.G5 = 'Chọn thời gian vắng: dưới 30 phút không hiện gì; từ 30 phút trở lên có thẻ đếm xong, chờ bạn, lỗi. Chạm một số để lọc danh sách.'
  const GX = window.NF.gfeat
  ALL.push(...GX)
  const gIcon = (f) => `<span class="fi" style="width:36px;height:36px;border-radius:10px;flex:none;display:grid;place-items:center;background:var(--c1-bg);color:var(--c1)">${I(f.ic, 18)}</span>`
  $('#g-list').innerHTML = GX.map((f) => `<a class="feat" href="#${f.to}"><div class="fh"><span class="fi">${I(f.ic, 24)}</span><div><span class="fid">${f.id}</span><strong>${esc(f.t)}</strong></div></div><p>${esc(f.d)}</p><div class="pills">${valPill(f.v)}<span class="pill p-c6">Công sức ${f.e}</span><span class="pill p-c3">${f.wave}</span></div></a>`).join('')
  $('#g-rv').innerHTML = window.NF.gReview.map(([id, v, e, r, k]) => {
    const f = GX.find((x) => x.id === id)
    return `<tr><td><b>${id}</b> ${esc(f.t)}</td><td>${f.wave}</td><td>${v}</td><td>${e}</td><td>${r}</td><td class="hide-sm">${k}</td></tr>`
  }).join('')
  const gcard = (id) => {
    const f = GX.find((x) => x.id === id)
    return `<article class="demo" id="demo-${id.toLowerCase()}" data-demo="${id}"><div class="demo-h"><strong>${gIcon(f)}${id} ${esc(f.t)}</strong><span class="pill p-c6">Công sức ${f.e}</span></div><p class="why">${WHY[id]}</p><div class="demo-stage" data-stage></div><div class="acts" data-acts></div></article>`
  }

  const ORDER = ['P1', 'P3', 'P6', 'P2', 'P4', 'P5', 'D3', 'D2']
  const ORDER2 = ['E1', 'E2', 'E5', 'E4', 'E6', 'E3']
  const ORDER3 = ['G1', 'G4', 'G3', 'G5', 'G2']
  const INIT = {}
  ;[...ORDER, ...ORDER2, ...ORDER3].forEach((id) => { INIT[id] = { ...D[id] } })
  const host = $('#demos')
  const host2 = $('#ex-demos')
  host.innerHTML = ORDER.map((id) => card(id, WHY[id], '', '')).join('')
  host2.innerHTML = ORDER2.map((id) => card(id, WHY[id], '', '')).join('')
  const host3 = $('#g-demos')
  host3.innerHTML = ORDER3.map(gcard).join('')
  function paint(id) {
    const el = $(`[data-demo="${id}"]`)
    $('[data-stage]', el).innerHTML = D[id].render(D[id].s)
    $('[data-acts]', el).innerHTML = D[id].acts()
  }
  ;[...ORDER, ...ORDER2, ...ORDER3].forEach(paint)
  const demoClick = (e) => {
    const el = e.target.closest('[data-demo]')
    if (!el) return
    const id = el.dataset.demo
    if (e.target.closest('[data-reset]')) { Object.assign(D[id], INIT[id], id === 'E6' ? { items: null } : id === 'G1' ? { res: [] } : {}); paint(id); return }
    if (commonClicks(e, $('.win-main', el) || $('[data-stage]', el))) return
    if (D[id].on?.(e, el)) paint(id)
  }
  host.addEventListener('click', demoClick)
  host2.addEventListener('click', demoClick)
  host3.addEventListener('click', demoClick)
  host3.addEventListener('keydown', (e) => { if (e.key === 'Escape' && D.G4.open) { D.G4.open = ''; paint('G4') } })
  let recT = 0, typeT = 0
  host.addEventListener('pointerdown', (e) => {
    const b = e.target.closest('[data-p4="mic"]')
    if (!b) return
    e.preventDefault()
    D.P4.s = 'rec'
    paint('P4')
    clearTimeout(recT)
  })
  const stopRec = () => {
    if (D.P4.s !== 'rec') return
    D.P4.s = 'idle'
    const full = 'Chạy test cho module auth rồi báo lại kết quả'
    D.P4.text = ''
    let i = 0
    clearInterval(typeT)
    typeT = setInterval(() => {
      i += 3
      D.P4.text = full.slice(0, i)
      paint('P4')
      if (i >= full.length) clearInterval(typeT)
    }, 40)
  }
  addEventListener('pointerup', stopRec)
  addEventListener('pointercancel', stopRec)
  host.addEventListener('keydown', (e) => {
    if ((e.key === 'Enter' || e.key === ' ') && e.target.closest('[data-p4="mic"]')) { e.preventDefault(); D.P4.s = 'rec'; paint('P4'); setTimeout(stopRec, 1200) }
  })
  setInterval(() => {
    const t = $('[data-p3t]')
    if (!t) return
    const [m, s] = t.textContent.split(':').map(Number)
    const n = m * 60 + s + 1
    t.textContent = `${Math.floor(n / 60)}:${String(n % 60).padStart(2, '0')}`
  }, 1000)

  const nav = $('.nav')
  const setPad = () => { document.documentElement.style.setProperty('--navh', `${nav.offsetHeight}px`) }
  setPad()
  addEventListener('resize', setPad)
  window.site?.tables()
  $('#yr').textContent = new Date().getFullYear()
})()
