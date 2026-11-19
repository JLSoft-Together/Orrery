(() => {
  const yr = document.getElementById('yr');
  if (yr) yr.textContent = new Date().getFullYear();
  const nav = document.querySelector('.nav');
  const pad = () => { if (nav) document.documentElement.style.scrollPaddingTop = (nav.offsetHeight + 16) + 'px'; };
  pad(); addEventListener('resize', pad);

  const tables = () => {
    document.querySelectorAll('.tw table').forEach(t => {
      const hs = [...t.querySelectorAll('thead th')].map(th => th.textContent.trim());
      t.querySelectorAll('tbody tr').forEach(tr => [...tr.children].forEach((td, i) => td.setAttribute('data-label', hs[i] || '')));
      const old = t.closest('.tw').querySelector('.more');
      if (old) old.remove();
      const rows = [...t.querySelectorAll('tbody tr')].filter(r => !r.classList.contains('hide'));
      t.querySelectorAll('tbody tr').forEach(r => r.classList.remove('fold'));
      const keep = +(t.dataset.keep || 8);
      if (rows.length <= keep + 2) return;
      const hidden = rows.slice(keep);
      hidden.forEach(r => r.classList.add('fold'));
      const b = document.createElement('button');
      b.type = 'button';
      b.className = 'more';
      b.setAttribute('aria-expanded', 'false');
      const label = open => b.textContent = open ? 'Thu gọn' : `Xem thêm ${hidden.length} dòng`;
      label(false);
      b.addEventListener('click', () => {
        const open = b.getAttribute('aria-expanded') !== 'true';
        hidden.forEach(r => r.classList.toggle('fold', !open));
        b.setAttribute('aria-expanded', String(open));
        label(open);
      });
      t.closest('.tw').appendChild(b);
    });
  };

  const spy = () => {
    const seg = document.getElementById('seg');
    if (!seg) return;
    const links = [...seg.querySelectorAll('a')].filter(a => a.getAttribute('href').startsWith('#'));
    const map = new Map(links.map(a => [a.getAttribute('href').slice(1), a]));
    const io = new IntersectionObserver(es => {
      es.forEach(e => {
        if (!e.isIntersecting) return;
        const a = map.get(e.target.id);
        if (!a) return;
        links.forEach(x => x.classList.toggle('on', x === a));
      });
    }, { rootMargin: '-45% 0px -50% 0px' });
    map.forEach((_, id) => { const s = document.getElementById(id); if (s) io.observe(s); });
  };

  const esc = s => String(s ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));

  window.site = { tables, spy, esc };
  spy();
})();
