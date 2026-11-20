(function () {
  var KEY = 'gon.v107';
  var STATES = [
    { v: 'giu', label: 'Giữ' },
    { v: 'an', label: 'Ẩn' },
    { v: 'gop', label: 'Gộp' },
    { v: 'bo', label: 'Bỏ' }
  ];
  var NAME = { giu: 'Giữ', an: 'Ẩn', gop: 'Gộp', bo: 'Bỏ' };

  var SURFACES = [
    {
      id: 'rail', title: 'Rail bên trái', img: 'rail.webp', side: true, cls: 'rail-shot',
      items: [
        [1, 'Logo O', 'bo', 'Chỉ trang trí, chiếm chỗ hàng đầu', [9.6, 2.9]],
        [2, 'Workspace / Giám sát', 'giu', 'Đổi view chính', [49, 2.9]],
        [3, 'Nút thu rail', 'an', 'Hiện khi hover mép rail, phím tắt vẫn chạy', [89, 2.9]],
        [4, 'Ô tìm session', 'gop', 'Thành icon kính lúp ở hàng đầu', [40, 7.6]],
        [5, 'Phím Ctrl+K trong ô tìm', 'bo', 'Đưa vào tooltip', [85, 7.6]],
        [6, 'Project / Trạng thái', 'an', 'Ít đổi, vào menu ... của rail', [42, 12.6]],
        [7, 'Nút + mở project', 'giu', 'Lối mở project chính', [90, 12.6]],
        [8, 'Mũi tên gập project', 'giu', 'Gập nhóm', [9, 17.4]],
        [9, 'Ô chữ cái project', 'gop', 'Thành chấm màu nhỏ cạnh tên', [20, 17.4]],
        [10, 'Tên project', 'giu', '', [36, 17.4]],
        [11, 'Branch cạnh project', 'an', 'Tooltip, pane đã có branch', [60, 17.4]],
        [12, 'Số session của project', 'an', 'Chỉ hiện khi gập', [85, 17.4]],
        [13, 'Menu ... project', 'giu', 'Đã chỉ hiện khi hover'],
        [14, 'Icon trạng thái hàng', 'giu', 'Icon + chữ + màu', [16, 21.8]],
        [15, 'Tên session', 'giu', '', [33, 21.8]],
        [16, 'Chip "Chờ bạn"', 'giu', 'Chữ trạng thái, đổi sang chữ cụ thể ở 17', [33, 23.9]],
        [17, 'Chi tiết "Cần duyệt"', 'gop', 'Thay chữ chip: "Cần duyệt"', [59, 23.9]],
        [18, 'Thời gian "chờ 2 phút"', 'gop', 'Vào chip: "Cần duyệt, 2 phút"', [70, 21.8]],
        [19, 'Chip Đụng file trên hàng', 'gop', 'Thành icon cảnh báo cạnh tên'],
        [20, 'Vòng context mini', 'an', 'Chỉ hiện khi gần đầy'],
        [21, 'Nút ngắm (hover)', 'bo', 'Trùng bấm vào hàng'],
        [22, 'Nút đóng (hover)', 'giu', 'Đã chỉ hiện khi hover'],
        [23, 'Dòng Công cụ N', 'giu', 'Đã gập sẵn'],
        [24, 'Chấm file chưa lưu ở Công cụ', 'giu', 'Tránh mất file'],
        [25, 'Pill cập nhật', 'giu', 'Hiếm, cần thấy'],
        [26, 'Chip Telegram', 'an', 'Vào menu ...'],
        [27, 'Chip điện thoại', 'an', 'Vào menu ..., Settings vẫn có'],
        [28, 'Icon trạng thái máy', 'an', 'Chỉ hiện khi mất kết nối hoặc quá tải', [9, 96.3]],
        [29, 'Thống kê', 'an', 'Vào menu ...', [27, 96.3]],
        [30, 'Lịch sử', 'giu', '', [47, 96.3]],
        [31, 'Cài đặt', 'giu', '', [66, 96.3]],
        [32, 'Menu ...', 'giu', 'Nơi gom các mục ẩn', [82, 96.3]]
      ]
    },
    {
      id: 'tabs', title: 'Dải tab và toolbar', img: 'tabs.webp',
      wf: [[[1, 'Orrery'], [4, 'O'], [5, 'api: migrate', 1], [13, '×'], [14, '+1'], [15, '+']], [[16, 'Bố cục'], [17, 'Inbox 4'], [18, 'Ctrl+Alt+Q'], [19, 'Explorer']]],
      items: [
        [1, 'Chip project đầu dải', 'bo', 'Rail đã chọn project, Ctrl+K đổi project', [5, 50]],
        [2, 'Chuông việc project khác', 'bo', 'Inbox đã đếm'],
        [3, 'Icon thư project khác', 'gop', 'Vào Inbox'],
        [4, 'Ô chữ cái tab + chấm trạng thái', 'giu', '', [9.2, 50]],
        [5, 'Tên tab', 'giu', '', [17, 50]],
        [6, 'Icon thư trên tab', 'gop', 'Vào Inbox'],
        [7, 'Icon Telegram trên tab', 'an', 'Vào tooltip tab'],
        [8, 'Chữ "2 pane"', 'bo', 'Nhìn bố cục là thấy'],
        [9, 'Icon zoom trên tab', 'giu', 'Báo có pane đang ẩn'],
        [10, 'Chấm chưa lưu', 'giu', 'Tránh mất file'],
        [11, 'Chip Lỗi trên tab', 'gop', 'Chấm đỏ ở ô chữ cái, chữ ở rail'],
        [12, 'Chip "chờ" trên tab', 'gop', 'Chấm vàng ở ô chữ cái, chữ ở Inbox'],
        [13, 'Nút × tab', 'giu', 'Chỉ tab đang mở', [48.5, 50]],
        [14, 'Nút +N tab ẩn', 'giu', '', [51.6, 50]],
        [15, 'Nút + tab mới', 'giu', '', [54.8, 50]],
        [16, 'Bố cục', 'an', 'Chỉ icon, chữ vào tooltip', [72, 50]],
        [17, 'Inbox + số', 'giu', 'Lối vào mọi việc chờ', [79.6, 50]],
        [18, 'Phím Ctrl+Alt+Q trong Inbox', 'bo', 'Đưa vào tooltip', [86.3, 50]],
        [19, 'Explorer', 'giu', '', [95, 50]]
      ]
    },
    {
      id: 'pane', title: 'Đầu pane', img: 'pane.webp',
      wf: [[[1, 'icon'], [2, 'Claude Code', 1], [6, 'Đụng file']], [[13, 'master'], [14, 'ghim'], [15, 'khối'], [16, 'trạng thái']], [[24, '...'], [26, 'zoom'], [27, 'chia'], [28, '×']]],
      items: [
        [1, 'Icon loại pane', 'giu', '', [4.3, 50]],
        [2, 'Tên pane', 'giu', '', [15.5, 50]],
        [3, 'Chip project riêng', 'giu', 'Chỉ hiện khi lệch project'],
        [4, 'Chip diff +/-', 'an', 'Vào menu ..., Changes đã có'],
        [5, 'Chip điện thoại đang xem', 'an', 'Vào menu ...'],
        [6, 'Đụng file', 'giu', 'Cảnh báo cần thấy', [37.7, 50]],
        [7, 'Chip trùng branch (overlap)', 'gop', 'Gộp với Đụng file'],
        [8, 'Vòng subagent', 'gop', 'Thành số trong chip trạng thái'],
        [9, 'Thanh context', 'gop', 'Một chỉ số context, hiện khi trên 70%'],
        [10, 'Số token context', 'bo', 'Trùng thanh context'],
        [11, 'Thanh context dưới đầu pane', 'gop', 'Gộp với 9'],
        [12, 'Gợi ý compact', 'gop', 'Thành nút của chỉ số context'],
        [13, 'Branch + menu worktree', 'giu', 'Lối duy nhất vào worktree', [52, 50]],
        [14, 'Kẹp ghim đính kèm', 'an', 'Vào menu ..., kéo thả file vẫn được', [61, 50]],
        [15, 'Công cụ session', 'an', 'Vào menu ...', [65.6, 50]],
        [16, 'Chip trạng thái session', 'giu', 'Icon + chữ + màu', [70.6, 50]],
        [17, 'Chữ hoạt động "Bash 1m"', 'giu', 'Biết agent đang làm gì'],
        [18, 'Chip tiến trình shell', 'gop', 'Vào chip trạng thái'],
        [19, 'Chip tin từ session khác', 'gop', 'Vào Inbox'],
        [20, 'Chip đang điều phối', 'gop', 'Vào chip trạng thái'],
        [21, 'Chip workflow agent', 'gop', 'Vào chip trạng thái'],
        [22, 'Chip hết limit, resume', 'giu', 'Có hành động'],
        [23, 'Chip bước compact', 'giu', 'Tạm thời, có tiến độ'],
        [24, 'Menu ...', 'giu', 'Nơi gom các mục ẩn', [76, 50]],
        [25, 'Số pane ẩn khi zoom', 'giu', 'Lối quay lại'],
        [26, 'Zoom', 'an', 'Hiện khi hover đầu pane, double click vẫn zoom', [82, 50]],
        [27, 'Chia pane', 'an', 'Hiện khi hover đầu pane', [88, 50]],
        [28, 'Đóng', 'giu', '', [94, 50]]
      ]
    },
    {
      id: 'footer', title: 'Footer quota',
      wf: [[[1, 'Claude 5 giờ 42%  7 ngày 18%', 1], [2, 'Sắp hết']], [[3, 'Chưa có hạn mức gói', 1], [4, 'Popover chi tiết']]],
      items: [
        [1, 'Thanh quota 5 giờ / 7 ngày', 'an', 'Bình thường 1 icon, hiện số khi trên 70%'],
        [2, 'Chữ "Sắp hết"', 'giu', 'Cảnh báo cần thấy'],
        [3, '"Chưa có hạn mức gói"', 'bo', 'Không có gì để báo'],
        [4, 'Popover chi tiết', 'giu', 'Bấm mới mở']
      ]
    },
    {
      id: 'monitor', title: 'Giám sát (Bảng)', img: 'monitor.webp',
      items: [
        [1, 'Câu tóm tắt', 'giu', '', [15.6, 3.4]],
        [2, 'Bảng / Quỹ đạo', 'giu', '', [39.4, 3.4]],
        [3, 'Nhóm theo Trạng thái / Project / Agent', 'gop', 'Gộp với 4 thành 1 menu Lọc', [74, 3.4]],
        [4, 'Lọc Tất cả dự án', 'gop', 'Cùng menu Lọc', [92.4, 3.4]],
        [5, 'Cột dọc Cần chủ nhân 0', 'an', 'Chỉ hiện khi khác 0', [3.1, 17]],
        [6, 'Cột dọc Xong hôm nay 0', 'an', 'Chỉ hiện khi khác 0', [97, 17]],
        [7, 'Ô chữ cái card', 'giu', '', [9.7, 16]],
        [8, 'Tên card', 'giu', '', [15, 15]],
        [9, 'Dòng "project, Claude Code"', 'gop', 'Còn icon agent, project đã nhóm', [19, 17.6]],
        [10, 'Chip subagent', 'giu', '', [43.5, 15.6]],
        [11, 'Chip trạng thái + tool', 'giu', '', [13.4, 20.6]],
        [12, 'Thời gian chạy', 'gop', 'Vào chip trạng thái', [10.9, 23.9]],
        [13, 'Branch', 'an', 'Vào tooltip', [18, 23.9]],
        [14, 'Thanh context', 'an', 'Chỉ hiện khi trên 70%', [18, 27.2]],
        [15, 'Lượt cuối', 'an', 'Vào tooltip'],
        [16, 'Mở trong Workspace', 'gop', 'Bấm card là mở', [15.8, 31.7]],
        [17, 'Xem diff', 'an', 'Hiện khi hover card', [28, 31.7]],
        [18, 'Dải Lệnh đang chạy', 'an', 'Chỉ hiện khi có lệnh chạy quá 30 giây', [7, 94.3]],
        [19, 'Rail session khi ở Giám sát', 'gop', 'Thu về icon, bảng đã đủ']
      ]
    },
    {
      id: 'inbox', title: 'Popover Inbox', img: 'inbox.webp', side: true,
      items: [
        [1, 'Tiêu đề Inbox', 'giu', '', [15, 7.4]],
        [2, 'Dải phím J/K, E, S', 'an', 'Vào icon ?', [76, 7.4]],
        [3, 'Nhóm Xin quyền / Câu hỏi / Lỗi', 'giu', '', [9, 19]],
        [4, 'Dòng 2 "Xin quyền, 2m"', 'gop', 'Chỉ còn thời gian, nhóm đã nói loại', [19, 31]],
        [5, 'Tên project mờ', 'giu', 'Biết session của project nào', [46, 26]]
      ]
    },
    {
      id: 'ctrlk', title: 'Ctrl+K',
      wf: [[[1, 'Tìm session, file, lệnh...', 1], [2, 'Esc']], [[3, 'Tất cả  Session  File  Lệnh  Project', 1]], [[4, 'Session 12'], [5, 'icon'], [6, 'Orrery, Claude Code', 1], [7, '2 phút trước'], [8, 'Shift+Enter tab mới']], [[9, '↑↓ Enter Shift+Enter Esc', 1], [10, '> @ / #', 1]]],
      items: [
        [1, 'Ô tìm', 'giu', ''],
        [2, 'Phím Esc', 'bo', 'Ai cũng biết Esc đóng'],
        [3, '5 tab phạm vi', 'giu', 'Lọc nhanh'],
        [4, 'Số đếm cạnh tên nhóm', 'bo', 'Không giúp chọn'],
        [5, 'Icon trạng thái hàng', 'giu', ''],
        [6, 'Dòng phụ hàng session', 'giu', ''],
        [7, '"x phút trước"', 'gop', 'Vào dòng phụ'],
        [8, 'Gợi ý Shift+Enter', 'giu', 'Chỉ hàng đang chọn'],
        [9, 'Dải phím ↑↓ Enter Esc', 'an', 'Vào icon ?'],
        [10, 'Dải tiền tố prefix', 'bo', 'Vào tooltip tab phạm vi, gõ prefix vẫn chạy']
      ]
    },
    {
      id: 'settings', title: 'Cài đặt (trang đầu)',
      wf: [[[1, 'Cài đặt'], [2, 'Tìm setting', 1], [9, '×']], [[3, '14 trang: Chung, Giao diện, Editor, Thông báo, Telegram, Remote, GitHub, Tích hợp, DeepSeek, Quyền, Dữ liệu, Giới thiệu, Workspace, Ổ đĩa', 1]], [[8, 'Chung: ngôn ngữ, khôi phục, awake, tips, Office, khởi động', 1]]],
      items: [
        [1, 'Tiêu đề Cài đặt', 'giu', ''],
        [2, 'Ô tìm setting', 'giu', 'Lối nhanh nhất'],
        [3, 'Thanh 14 trang', 'gop', 'Còn 6 nhóm (4 tới 7)'],
        [4, 'Thông báo + Telegram + Remote', 'gop', '1 trang Thông báo và điện thoại'],
        [5, 'GitHub + Tích hợp + DeepSeek', 'gop', '1 trang Tài khoản và tích hợp'],
        [6, 'Dữ liệu + Ổ đĩa + Quyền', 'gop', '1 trang Dữ liệu'],
        [7, 'Editor + Workspace', 'gop', '1 trang Workspace'],
        [8, 'Office trong Chung', 'an', 'Office đang tạm dừng, dời xuống cuối'],
        [9, 'Nút đóng', 'giu', '']
      ]
    },
    {
      id: 'empty', title: 'Workspace trống',
      wf: [[[1, 'Chưa có tab nào + câu mô tả', 1]], [[2, 'Danh sách khôi phục phiên trước', 1]]],
      items: [
        [1, 'Tiêu đề + câu mô tả', 'gop', 'Còn 1 dòng'],
        [2, 'Danh sách khôi phục', 'giu', 'Việc hay làm nhất khi mở app']
      ]
    }
  ];

  var choice = {};
  try {
    var raw = window.localStorage.getItem(KEY);
    if (raw) choice = JSON.parse(raw) || {};
  } catch (e) {
    choice = {};
  }

  function save() {
    try { window.localStorage.setItem(KEY, JSON.stringify(choice)); } catch (e) {}
  }
  function key(s, it) { return s.id + ':' + it[0]; }
  function cur(s, it) { var v = choice[key(s, it)]; return NAME[v] ? v : it[2]; }

  function el(tag, cls, text) {
    var n = document.createElement(tag);
    if (cls) n.className = cls;
    if (text != null) n.textContent = text;
    return n;
  }

  var main = document.getElementById('main');

  function render() {
    main.textContent = '';
    SURFACES.forEach(function (s) {
      var sec = el('section', 'surf');
      sec.id = s.id;
      sec.setAttribute('aria-labelledby', 'h-' + s.id);
      var head = el('div', 'surf-h');
      var h = el('h2', null, s.title);
      h.id = 'h-' + s.id;
      head.appendChild(h);
      head.appendChild(el('span', 'cnt', s.items.length + ' phần tử'));
      var mini = el('span', 'mini');
      mini.setAttribute('data-mini', s.id);
      head.appendChild(mini);
      sec.appendChild(head);

      var grid = el('div', 'grid' + (s.side ? ' side' : ''));
      var shot = el('div', 'shot' + (s.cls ? ' ' + s.cls : '') + (s.img && s.wf ? ' both' : ''));
      if (s.img) {
        var frame = el('div', 'frame');
        var img = el('img');
        img.src = s.img;
        img.alt = 'Ảnh ' + s.title;
        frame.appendChild(img);
        s.items.forEach(function (it) {
          if (!it[4]) return;
          var m = el('span', 'mk', String(it[0]));
          m.style.left = it[4][0] + '%';
          m.style.top = it[4][1] + '%';
          m.setAttribute('data-mk', key(s, it));
          m.setAttribute('aria-hidden', 'true');
          frame.appendChild(m);
        });
        shot.appendChild(frame);
      }
      if (s.wf) {
        var wf = el('div', 'wf');
        wf.setAttribute('aria-hidden', 'true');
        s.wf.forEach(function (row) {
          var r = el('div', 'wf-row');
          row.forEach(function (b) {
            var box = el('span', 'wf-b' + (b[2] ? ' grow' : ''));
            var n = el('span', 'n', String(b[0]));
            n.setAttribute('data-mk', s.id + ':' + b[0]);
            box.appendChild(n);
            box.appendChild(el('span', 'l', b[1]));
            r.appendChild(box);
          });
          wf.appendChild(r);
        });
        shot.appendChild(wf);
      }
      grid.appendChild(shot);

      var ul = el('ul', 'list');
      s.items.forEach(function (it) {
        var li = el('li', 'it');
        li.appendChild(el('span', 'num', String(it[0])));
        var lab = el('span', 'lab', it[1]);
        var chg = el('span', 'chg');
        chg.setAttribute('data-chg', key(s, it));
        lab.appendChild(chg);
        li.appendChild(lab);
        var pill = el('span', 'pill');
        pill.setAttribute('role', 'group');
        pill.setAttribute('aria-label', it[1]);
        STATES.forEach(function (st) {
          var b = el('button', null, st.label);
          b.type = 'button';
          b.setAttribute('data-v', st.v);
          b.setAttribute('data-k', key(s, it));
          b.addEventListener('click', function () {
            if (st.v === it[2]) delete choice[key(s, it)];
            else choice[key(s, it)] = st.v;
            save();
            sync();
          });
          pill.appendChild(b);
        });
        li.appendChild(pill);
        if (it[3]) li.appendChild(el('span', 'why', it[3]));
        ul.appendChild(li);
      });
      grid.appendChild(ul);
      sec.appendChild(grid);
      main.appendChild(sec);
    });
  }

  function tags(counts, total) {
    var f = document.createDocumentFragment();
    if (total != null) {
      var a = el('span', 'tag t-all');
      a.appendChild(document.createTextNode('Tổng '));
      a.appendChild(el('b', null, String(total)));
      f.appendChild(a);
    }
    STATES.forEach(function (st) {
      var t = el('span', 'tag t-' + st.v);
      t.appendChild(document.createTextNode(st.label + ' '));
      t.appendChild(el('b', null, String(counts[st.v])));
      f.appendChild(t);
    });
    return f;
  }

  function sync() {
    var all = { giu: 0, an: 0, gop: 0, bo: 0 };
    var total = 0;
    var changed = 0;
    SURFACES.forEach(function (s) {
      var c = { giu: 0, an: 0, gop: 0, bo: 0 };
      s.items.forEach(function (it) {
        var v = cur(s, it);
        var k = key(s, it);
        c[v]++; all[v]++; total++;
        var btns = main.querySelectorAll('button[data-k="' + k + '"]');
        Array.prototype.forEach.call(btns, function (b) {
          b.setAttribute('aria-pressed', b.getAttribute('data-v') === v ? 'true' : 'false');
        });
        Array.prototype.forEach.call(main.querySelectorAll('[data-mk="' + k + '"]'), function (mk) {
          mk.className = (mk.classList.contains('n') ? 'n' : 'mk') + ' s-' + v;
        });
        var chg = main.querySelector('[data-chg="' + k + '"]');
        var diff = v !== it[2];
        if (diff) changed++;
        if (chg) chg.textContent = diff ? 'đã đổi' : '';
      });
      var mini = main.querySelector('[data-mini="' + s.id + '"]');
      if (mini) { mini.textContent = ''; mini.appendChild(tags(c)); }
    });
    var tally = document.getElementById('tally');
    tally.textContent = '';
    tally.appendChild(tags(all, total));
    if (changed) {
      var x = el('span', 'tag t-all', 'Chủ nhân đổi ' + changed);
      tally.appendChild(x);
    }
  }

  function summary() {
    var lines = ['Gọn UI v107, desktop'];
    var changes = [];
    SURFACES.forEach(function (s) {
      var by = { giu: [], an: [], gop: [], bo: [] };
      s.items.forEach(function (it) {
        var v = cur(s, it);
        by[v].push(it[0] + ' ' + it[1]);
        if (v !== it[2]) changes.push(s.title + ' ' + it[0] + ' ' + it[1] + ': ' + NAME[it[2]] + ' -> ' + NAME[v]);
      });
      lines.push('');
      lines.push('[' + s.title + ']');
      ['bo', 'an', 'gop', 'giu'].forEach(function (v) {
        if (by[v].length) lines.push(NAME[v] + ': ' + by[v].join('; '));
      });
    });
    lines.push('');
    lines.push(changes.length ? 'Chủ nhân đổi so với đề xuất:' : 'Chủ nhân giữ nguyên đề xuất.');
    changes.forEach(function (c) { lines.push('- ' + c); });
    return lines.join('\n');
  }

  var toastEl = document.getElementById('toast');
  var toastT = 0;
  function toast(msg) {
    toastEl.textContent = msg;
    toastEl.classList.add('on');
    clearTimeout(toastT);
    toastT = setTimeout(function () { toastEl.classList.remove('on'); }, 1800);
  }

  function fallbackCopy(text) {
    var ta = document.createElement('textarea');
    ta.value = text;
    ta.setAttribute('readonly', '');
    ta.style.position = 'fixed';
    ta.style.top = '-1000px';
    document.body.appendChild(ta);
    ta.select();
    var ok = false;
    try { ok = document.execCommand('copy'); } catch (e) { ok = false; }
    document.body.removeChild(ta);
    return ok;
  }

  document.getElementById('copy').addEventListener('click', function () {
    var text = summary();
    window.__gonLast = text;
    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(text).then(function () { toast('Đã copy kết quả'); }, function () {
        toast(fallbackCopy(text) ? 'Đã copy kết quả' : 'Không copy được');
      });
    } else {
      toast(fallbackCopy(text) ? 'Đã copy kết quả' : 'Không copy được');
    }
  });

  document.getElementById('reset').addEventListener('click', function () {
    choice = {};
    save();
    sync();
    toast('Đã về đề xuất');
  });

  render();
  sync();
})();
