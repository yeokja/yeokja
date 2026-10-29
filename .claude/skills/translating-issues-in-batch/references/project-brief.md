# 프로젝트 브리프 템플릿 (프로젝트마다 서브에이전트 1개)

main 체크아웃을 그대로 쓴다. 다른 에이전트가 같은 체크아웃의 다른 `projects/<name>/`에서
동시에 작업한다. yeokja CLI는 `../../target/release/yeokja`; **`cargo build`를 돌리지 않는다.**
provider는 `claude_code`/`claude-opus-5-5`, `[translation] concurrency = 6`.

반드시 읽을 것: `AGENTS.md`, `docs/translation-project-checklist.md`, 유사 프로젝트의
`yeokja.toml`·scripts·README·`nix/projects/<name>.nix`, 이 프로젝트의 조사 보고서, 관련 코어 spec.

관례: spec(`docs/superpowers/specs/<date>-<name>-korean-translation-design.md`)과
plan(`docs/superpowers/plans/...`) 먼저. 문서는 spec이지 일지가 아니다. 결정은 스스로 내리고
근거를 spec에 적는다. 서브모듈은 호출 측이 이미 고정했다.

순서: 시험 번역 1파일(원문이 큰 파일 하나면 앞부분을 잘라 scratch 사본에서) → 재구성 diff·재시도율·문체 검토 → 전체 translate →
`evaluate --mechanical-only` 0건 → `terms build --dry-run` → `terms build` → terms.toml 검토 →
`glossary adopt --prefer HEAD` → 남은 세그먼트 translate → `status --check` → nix devShell로
실제 `yeokja build html` → 링크·앵커·제목 수 검사 스크립트 → 브라우저 확인 → README(AGENTS.md
출처 표기 규칙) → `bash .github/scripts/rebuild-translations.sh <name>` 통과.

**건드리지 말 것**(호출 측이 통합): `.gitmodules`, `.github/pages-projects.json`,
`.github/scripts/stage-pages.sh`, `.github/scripts/rebuild-translations.sh`, `.github/workflows/*`,
루트 `LICENSE.md`, `site/index.html`, `crates/**`, `Cargo.*`, 다른 `projects/*`, `AGENTS.md`,
`docs/translation-project-checklist.md`. `git add`/commit/push/stash/checkout 금지.

보고(마지막 메시지): 만든 파일 목록, 세그먼트·재시도·evaluate·terms 수치, 빌드·검사 결과,
그리고 **그대로 붙일 수 있는 형태의** 공유 파일 변경(pages-projects.json 항목, stage-pages.sh
줄, rebuild-translations.sh case 분기, LICENSE.md 표 한 줄, site/index.html 카드 — 기존 카드 형식 그대로: 어울리는 기존 주제
section에, `work-meta`에 원문 링크·라이선스 태그·`기계 번역` 태그; 새 주제가 필요하면 filter
버튼과 section도), 열린 문제, 사람만 할 수 있는 항목.
