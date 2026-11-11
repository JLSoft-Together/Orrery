export const APP_SCHEME = "orrery-remote";
export const ANDROID_PACKAGE = "dev.ccm.orrery.remote";

interface Strings {
  lang: string;
  title: string;
  hint: string;
  open: string;
  store: string;
}

const VI: Strings = {
  lang: "vi",
  title: "Mở trong Orrery Remote",
  hint: "Nếu app chưa mở, hãy cài Orrery Remote rồi bấm lại nút này.",
  open: "Mở app",
  store: "Mở bằng Android",
};

const EN: Strings = {
  lang: "en",
  title: "Open in Orrery Remote",
  hint: "If the app does not open, install Orrery Remote and tap the button again.",
  open: "Open app",
  store: "Open on Android",
};

function pickStrings(acceptLanguage: string | null): Strings {
  const first = (acceptLanguage ?? "").split(",")[0]?.trim().toLowerCase() ?? "";
  return first.startsWith("vi") ? VI : EN;
}

function escapeHtml(text: string): string {
  return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

export function appLink(machineId: string, paneKey: string): string {
  return `${APP_SCHEME}://m/${encodeURIComponent(machineId)}/p/${encodeURIComponent(paneKey)}`;
}

export function androidIntent(machineId: string, paneKey: string): string {
  return `intent://m/${encodeURIComponent(machineId)}/p/${encodeURIComponent(paneKey)}#Intent;scheme=${APP_SCHEME};package=${ANDROID_PACKAGE};end`;
}

export function bouncePage(machineId: string, paneKey: string, acceptLanguage: string | null, userAgent: string | null): Response {
  const s = pickStrings(acceptLanguage);
  const android = /android/i.test(userAgent ?? "");
  const link = escapeHtml(appLink(machineId, paneKey));
  const primary = android ? escapeHtml(androidIntent(machineId, paneKey)) : link;
  const secondary = android ? `<p><a class="alt" href="${link}">${escapeHtml(s.store)}</a></p>` : "";
  const html = `<!doctype html>
<html lang="${s.lang}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<meta name="robots" content="noindex,nofollow">
<meta name="referrer" content="no-referrer">
<title>${escapeHtml(s.title)}</title>
<style>
body{margin:0;min-height:100vh;display:flex;align-items:center;justify-content:center;font:16px/1.5 system-ui,sans-serif;background:#f5f5f7;color:#1d1d1f}
main{max-width:22rem;padding:2rem;text-align:center}
a.go{display:block;padding:.9rem 1.2rem;border-radius:.8rem;background:#0a84ff;color:#fff;text-decoration:none;font-weight:600}
a.alt{color:#0a84ff}
p{color:#6e6e73}
</style>
</head>
<body>
<main>
<h1>${escapeHtml(s.title)}</h1>
<a class="go" href="${primary}">${escapeHtml(s.open)}</a>
${secondary}
<p>${escapeHtml(s.hint)}</p>
</main>
</body>
</html>`;
  return new Response(html, {
    status: 200,
    headers: {
      "content-type": "text/html; charset=utf-8",
      "cache-control": "no-store",
      "content-security-policy": "default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'",
      "x-content-type-options": "nosniff",
      "referrer-policy": "no-referrer",
      vary: "Accept-Language, User-Agent",
    },
  });
}

export function assetLinks(certs: string | undefined): Response {
  const fingerprints = (certs ?? "")
    .split(",")
    .map((c) => c.trim())
    .filter((c) => /^([0-9A-Fa-f]{2}:){31}[0-9A-Fa-f]{2}$/.test(c));
  if (fingerprints.length === 0) return new Response("not found", { status: 404 });
  const body = [
    {
      relation: ["delegate_permission/common.handle_all_urls"],
      target: { namespace: "android_app", package_name: ANDROID_PACKAGE, sha256_cert_fingerprints: fingerprints },
    },
  ];
  return new Response(JSON.stringify(body), { headers: { "content-type": "application/json", "cache-control": "public, max-age=3600" } });
}

export function appleSiteAssociation(appId: string | undefined): Response {
  const ids = (appId ?? "")
    .split(",")
    .map((c) => c.trim())
    .filter((c) => /^[A-Z0-9]{10}\.[A-Za-z0-9.-]+$/.test(c));
  const body = { applinks: { details: ids.length > 0 ? [{ appIDs: ids, components: [{ "/": "/o/*" }] }] : [] } };
  return new Response(JSON.stringify(body), { headers: { "content-type": "application/json", "cache-control": "public, max-age=3600" } });
}
