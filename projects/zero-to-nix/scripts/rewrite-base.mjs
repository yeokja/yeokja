// Astro의 `base` 설정은 Astro 자신이 만드는 경로(_astro/ 자산, 사이트맵)에만
// 접두사를 붙입니다. MDX 본문의 `[텍스트](/concepts/nix)`나 컴포넌트에 손으로
// 적힌 `href="/start"` 같은 루트 절대 경로는 그대로 남아, 하위 경로에 배포하면
// yeokja.moreal.dev의 루트를 가리키게 됩니다. 빌드 결과물 전체를 한 번 훑어
// 아직 접두사가 없는 루트 절대 URL에 base를 붙입니다.
//
// 사용법: node rewrite-base.mjs <site-dir> <base>
//   예: node rewrite-base.mjs site /zero-to-nix

import { readdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";

const [siteDir, rawBase] = process.argv.slice(2);
if (!siteDir || !rawBase) {
  console.error("usage: rewrite-base.mjs <site-dir> <base>");
  process.exit(2);
}
const base = rawBase.replace(/\/+$/, "");
if (!base.startsWith("/")) {
  console.error(`base must start with '/': ${rawBase}`);
  process.exit(2);
}

// `/`로 시작하지만 `//`(프로토콜 상대)나 이미 base가 붙은 경로가 아닌 값.
const needsPrefix = (path) =>
  path.startsWith("/") &&
  !path.startsWith("//") &&
  path !== base &&
  !path.startsWith(`${base}/`);

const prefix = (path) => `${base}${path}`;

// HTML 속성(href, src, srcset, action, poster, content 등)의 따옴표 값.
const HTML_ATTRIBUTE = /\b(href|src|srcset|action|poster)=("|')([^"']*)\2/g;

const rewriteHtml = (text) =>
  text.replace(HTML_ATTRIBUTE, (match, name, quote, value) => {
    if (name === "srcset") {
      const rewritten = value
        .split(",")
        .map((candidate) => {
          const trimmed = candidate.trim();
          const [url, ...descriptor] = trimmed.split(/\s+/);
          const next = needsPrefix(url) ? prefix(url) : url;
          return [next, ...descriptor].join(" ");
        })
        .join(", ");
      return `${name}=${quote}${rewritten}${quote}`;
    }
    return needsPrefix(value) ? `${name}=${quote}${prefix(value)}${quote}` : match;
  });

// Markdown 링크 `](/path)` — llms*.txt와 페이지별 .md 원문에 쓰입니다.
const MARKDOWN_LINK = /\]\((\/[^)\s]*)\)/g;
const rewriteMarkdown = (text) =>
  text.replace(MARKDOWN_LINK, (match, path) =>
    needsPrefix(path) ? `](${prefix(path)})` : match,
  );

const REWRITERS = {
  ".html": rewriteHtml,
  ".md": rewriteMarkdown,
  ".txt": rewriteMarkdown,
};

async function* walk(dir) {
  for (const entry of await readdir(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) yield* walk(path);
    else yield path;
  }
}

let rewritten = 0;
for await (const path of walk(siteDir)) {
  const ext = Object.keys(REWRITERS).find((suffix) => path.endsWith(suffix));
  if (!ext) continue;
  const before = await readFile(path, "utf8");
  const after = REWRITERS[ext](before);
  if (after !== before) {
    await writeFile(path, after);
    rewritten += 1;
  }
}
console.log(`rewrite-base: prefixed root-absolute URLs with ${base} in ${rewritten} file(s)`);
