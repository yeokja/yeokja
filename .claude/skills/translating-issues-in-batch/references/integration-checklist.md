# 통합 체크리스트 (호출 측, 프로젝트마다)

- [ ] 서브모듈 고정 커밋(직렬로 add; `.gitmodules`에 `shallow = true`; 중첩 서브모듈은 필요한
      것만 `--depth 1`로 init)
- [ ] `.github/pages-projects.json` 항목(`project`, `target`, `artifact`, `artifact_path`;
      원문 안의 서브모듈이 빌드에 필요하면 `"nested_submodules": "<경로> <경로>"`;
      git 전체 이력이 필요하면 `unshallow`)
- [ ] `.github/scripts/stage-pages.sh`에 `overlay_site "dist-<name>" "site" "<name>"`
- [ ] `.github/scripts/rebuild-translations.sh` case 분기: 범위 검사 → `status --check <src>` →
      `translate <src>` 순서(CI가 provider를 부르지 않게 **check가 먼저**)
- [ ] 루트 `LICENSE.md` 표 한 줄(알파벳 순)
- [ ] `site/index.html`: 카드, 필요하면 주제 filter 버튼 + section, `#result-count` 편수
- [ ] `nix/projects/<name>.nix` 존재
- [ ] README 출처 표기(AGENTS.md 규칙: yeokja 링크, state에 기록된 실제 모델, 학습 비허용 문장)
- [ ] 프로젝트 `.gitignore`가 `/ko/`처럼 루트 고정인지(`ko/`는 `overlay/ko/`도 가린다)

검증(push 전, 전부 호출 측이 직접):
- [ ] 코어를 바꿨으면 `cargo test --workspace`, `cargo clippy --workspace --all-targets`,
      `cargo build --release -p yeokja-cli`
- [ ] `.github/pages-projects.json`의 **모든** 프로젝트에 `rebuild-translations.sh` 실행 → exit 0,
      `git status`에 state 변화 없음(수십 분 걸리므로 백그라운드로):
      `for p in $(jq -r '.[].project' .github/pages-projects.json | sort -u); do bash .github/scripts/rebuild-translations.sh $p >/dev/null 2>&1 && echo "$p ok" || echo "$p FAIL"; done`
- [ ] 새 프로젝트마다 `nix develop path:../../nix#<name> -c ../../target/release/yeokja build html`
      재현(에이전트 보고만 믿지 않음)
- [ ] `python3 -m unittest`로 `.github/scripts/test_*.py`
- [ ] 커밋 단위: 서브모듈 고정 / 코어(브랜치 머지) / 프로젝트 하나 / pages·site 배선. 서명은
      SSH(Touch ID). 사용자가 없으면 scratch `commits/replay.sh`(`msg-N.txt` + `git add`/`commit -F`)로
      모아 두고 보고. push도 같은 키라 사용자가 있어야 한다.
- [ ] push 후 `gh run watch <run> --exit-status`, 실패 잡은 `gh run view --job <id> --log`로 원인,
      새 경로 `curl -o /dev/null -w '%{http_code}' https://yeokja.moreal.dev/<name>/`(배포
      도메인은 랜딩·README에 있음), 랜딩 링크 확인. Deploy 전체가 success여도 실패한 프로젝트는
      이전 배포분을 보존하므로 **잡 단위** conclusion을 본다
- [ ] 완료된 이슈는 배포 주소 코멘트와 함께 `gh issue close`(사용자가 지시했거나 확인한 경우);
      보류 이슈는 사유를 보고하고 사용자 확인 뒤 코멘트·닫기
