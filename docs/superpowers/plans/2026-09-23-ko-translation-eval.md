# 한국어 번역 eval 구현 계획

**Goal:** `evals/ko-translation/README.md`의 설계대로 동결된 eval 세트를 만들고,
후보 모델의 번역·자동 관문·쌍대 채점·사람 판정·보고서를 한 도구로 돌린다.

**Architecture:** 운영 요청을 재생할 수 있도록 orchestrator의 요청 생성부를
공개 함수로 떼어 내고, 인라인 태그 타입에 serde를 붙여 요청을 그대로 직렬화한다.
새 크레이트 `crates/eval`(바이너리 `yeokja-eval`)이 추출·실행·관문·채점·내보내기를
맡는다. 원문 서브모듈은 추출 때만 필요하고, 실행부터는 동결된 항목만 읽는다.

**Spec:** `evals/ko-translation/README.md`

## Global Constraints

- 운영 번역 동작은 바이트 하나 바뀌지 않는다(기존 테스트 전부 통과, 리팩터링만).
- 항목은 운영의 `batch_segments`를 그대로 따른 요청 하나다. 채점 단위는 그 안의
  블록이다.
- 발췌는 라이선스 파일 단위로만 쓴다. pypy-eu-reports와 napkin은 없고, peps는
  번역 대상 PEP 중 OPL이 아닌 것만 쓴다.
- API 키는 환경 변수로만 받는다. 로그와 `run.toml`에 남기지 않는다.
- 커밋은 Secretive(Touch ID) SSH 서명. 우회 금지.

---

### Task 1: 요청 생성부 공개와 직렬화

**Files:** `crates/translate/src/orchestrator.rs`, `crates/translate/src/provider.rs`,
`crates/translate/src/inline/markdown.rs`, `crates/core/src/parser.rs`

- `translate_block`의 요청 조립(태그화, `paragraphs`, 용어집, 템플릿)을
  `fn build_request(...) -> (TranslateRequest, Option<InlineBatch>)`로 떼어 내고
  `translate_block`이 그것을 부르게 한다.
- `pub fn plan_requests(file, config, glossary, parser_factory) -> Vec<PlannedRequest>`:
  파일의 번역 대상 세그먼트 전부를 미번역으로 보고, 운영과 같은 `group_by_block` →
  `batch_block_groups` → `build_request`로 요청을 만든다. `PlannedRequest`는 요청,
  인라인 배치, 블록별 세그먼트 ID와 원문, 블록 `raw_content`를 담는다.
- `TranslateRequest`, `Markup`, `InlineBatch`, `Tagged`, `Tag`, `TagKind`,
  `RoleRef`, `Position`, `Dialect`, `DocContext`에 `Serialize`/`Deserialize`.
- 테스트: `plan_requests`가 만든 요청이 `translate_path`가 보내는 요청과 같다
  (모의 프로바이더로 받은 요청을 비교). 직렬화 왕복 후 `read`/`render` 결과가 같다.

### Task 2: `yeokja-eval extract`

**Files:** `crates/eval/` (새 크레이트), 워크스페이스 `Cargo.toml`

- 각 프로젝트 디렉터리에서 설정·용어집을 읽어 `plan_requests`로 모든 요청을 만들고,
  고정 시드(기본 `20260923`)로 층화 추출한다.
  - 프로젝트당 2요청부터 돌아가며 채워 약 60요청, 프로젝트당 최대 4요청, 파일당 1요청.
  - 요청의 모든 세그먼트가 사소하면(20자 미만, 산문 글자 12자 미만) 제외.
  - 라이선스 규칙은 `v1/sources.toml`(버킷, SPDX, 저작자, 범위, peps의 OPL 제외).
    napkin은 발췌가 GPL LaTeX 소스에서 나오므로 제외.
- 어려운 항목: 지정한 (파일, 세그먼트 ID) 목록의 세그먼트가 든 요청을 추가하고,
  git에서 고친 번역과 원래 번역을 찾아 `reference`/`known_bad`에 넣는다.
- 합성 항목: `evals/ko-translation/v1/synthetic/`의 미니 프로젝트를 같은 방식으로
  전부 뽑는다(표본 추출 없음). 블록별 `probe`는 그 프로젝트의 `probes.toml`에서.
- 출력: 라이선스별 `items.*.jsonl`, `glossaries/*.toml`, `MANIFEST.md`.
  같은 입력이면 바이트까지 같은 출력(정렬된 키, 안정된 순서).

### Task 3: 합성 미니 프로젝트

**Files:** `evals/ko-translation/v1/synthetic/`

- 직접 쓴 영어 원문으로 설계 문서 4.2절의 유형을 약 50블록 만든다. Markdown
  (`inline_tags = true`), reStructuredText, LaTeX 소스를 둔다.
- `probes.toml`에 블록별 시험 대상을 적는다. 라이선스는 `MIT OR Apache-2.0`.

### Task 4: `yeokja-eval run`

- 입력: 항목 파일들, 후보 `[provider]` 설정(TOML), 출력 디렉터리.
- 항목마다 요청을 역직렬화해 `translate_with_evaluation_observed`로 보낸다.
  평가기는 운영과 같은 `evaluators_for`(StyleEvaluator는 후보 자신의 모델).
  첫 시도 번역은 관찰자(`PipelineEvent`)와 프로바이더 래퍼로 가로채 저장한다.
- 출력: 라이선스별 `outputs.*.jsonl`(블록별 첫 시도·최종 번역, 시도 횟수, 이슈,
  소요 시간), `run.toml`. 중단 후 재실행하면 끝난 항목은 건너뛴다.
- `--repeat 3 --subset 30`: 비결정성 측정용.

### Task 5: `yeokja-eval gate`

- 블록마다 기존 평가기(용어집·링크·서식·어미), `inline::audit`, `alignment`,
  GFM 경고 마커 검사, 새 잘림 검사를 돌려 `gates.jsonl`을 쓴다.
- 잘림 검사: 세그먼트별 번역/원문 길이 비율이 기존 state 번역 분포의 0.5 백분위
  미만이거나 원문 문장 수 대비 번역 문장 수가 절반 미만이면 실패. 분포는 extract가
  계산해 `MANIFEST.md`와 `v1/truncation.toml`에 동결한다.

### Task 6: `yeokja-eval judge`

- 기준선 대 후보, 두 관문을 모두 통과한 블록을 대상으로 한다. 판정용 블록 표본
  (실제 약 150, 합성 전부, 어려운 항목 전부)은 시드로 고정한다.
- 채점자: `codex`(`gpt-6-astra`), `gemini`(최신 안정판 Flash, 실행 시 모델 목록
  API로 확인해 기록). 같은 쌍을 A/B 순서를 바꿔 두 번 묻고 엇갈리면 동점.
- 판정 프롬프트는 설계 문서 6.2·6.3절의 기준과 JSON 응답 형식을 담는다.
- 출력: `judgments/<run>/pairs.jsonl`, `blind-map.json`.

### Task 7: 사람 판정 페이지

- 판정 표본 중 30블록을 층화 추출해 Artifact(비공개, db capability)로 게시한다.
  모델 이름 없이 A/B만. `yeokja-eval export-human`이 db 내보내기를
  `human.jsonl`로 바꾼다.

### Task 8: `yeokja-eval report`

- 관문 통과율(첫 시도/최종), 채점자별 승률과 95% 신뢰 구간(윌슨), 번역투 승률,
  사람–채점자 일치율, 프로젝트·파서별 분해, 비용·시간을 `reports/<run>.md`로 쓴다.
  설계 문서 7절의 판정 기준을 그대로 적용해 결론 줄을 만든다.
