# Aqueous 아카이브 형식 명세 (한국어 번역)

Kang Seonghoon이 쓰는 *Aqueous Archive Format* 명세
([lifthrasiir/aqueous](https://github.com/lifthrasiir/aqueous)의 `spec/index.bs`,
편집자 초안)를 한국어로 옮긴 비공식 번역입니다.
[yeokja](https://github.com/yeokja/yeokja)와 함께 Anthropic 사의 `claude-opus-5-5`
모델을 활용하여 번역되었으며 학습을 모두 비허용한 상태로 작업하였습니다.

## 스냅샷과 비공식 고지

- 원문: `upstream/` 서브모듈, 커밋 `3d37dae5431c2b23a41ded4e06a46276eb0565b4`
  (2026-10-29, `main`). 원문 페이지는
  <https://lifthrasiir.github.io/aqueous/spec/>입니다.
- 원문은 매주 크게 바뀌는 편집자 초안이므로 이 번역은 위 커밋 시점의 내용만
  반영합니다. 빌드된 페이지의 머리말 경고 상자가 스냅샷 커밋과 날짜, 원문
  링크를 보입니다(빌드 스크립트가 서브모듈의 커밋에서 읽습니다).
- 저자의 공식 번역이 아니며 저자가 검토하지 않았습니다. 규범적 효력은 원문에만
  있습니다.

## 범위

- 번역 대상: `upstream/spec/index.bs` 한 파일(`bikeshed` 파서, 2,471 세그먼트).
  본문 산문, 제목, 표 머리글, 노트·예시·이슈, `<dfn>` 산문 용어, `[=term=]` 자동
  링크와 `[[#id|text]]` 절 참조의 표시 글자, 메타데이터의 `Abstract:`를
  번역합니다.
- 원문 그대로 두는 것: 나머지 메타데이터, `<pre>`(CDDL, 참고 문헌, 앵커, 그림
  포함), 16진 상수, 식별자형 정의(`u8`, `leb128`, `offset`, `HEADER_TYPE`,
  `block_start`, `aqr_send` …), 명령 이름(Nop, Literal, Swap, Slice, Apply, Concat,
  Pack, Split, Patch, Checksum), 레지스트리 표의 이름·해시·데이터, 조건 키워드
  `Optional`/`If`/`Depends on`, Bikeshed가 만드는 저작권 문단.
- 번역된 정의에는 파서가 `lt="원문"`을 붙여 `[=원문=]` 자동 링크가 그대로
  풀립니다. 용어집은 `glossary.toml`, 프로젝트 용어 기준은 `terms.toml`에 있으며
  원문 저자가 `docs/`에서 쓰는 한국어(아카이브, 블록, 세그먼트, 체크섬, 바이트열,
  사전, 엔트리 …)를 우선합니다.
- RFC 2119 키워드는 고정 어구로 옮깁니다: MUST → ~해야 합니다, MUST NOT →
  ~해서는 안 됩니다, SHOULD → ~하는 것이 좋습니다, SHOULD NOT → ~하지 않는 것이
  좋습니다, MAY → ~할 수 있습니다, REQUIRED → 필수, OPTIONAL → 선택.
- 원문은 한 줄에 한 문장을 쓰지만 번역 문단은 한 줄로 나옵니다. Bikeshed 출력은
  같습니다.

## 알려진 한계

- 색인("이 명세가 정의하는 용어")은 정의의 `lt`(원문 용어)로 정렬·표시되므로 영어
  용어로 보입니다. 링크는 번역된 정의로 갑니다.
- 구조 표의 필드 이름(`<td><dfn>size</dfn>`)은 식별자라 영문으로 두며,
  `glossary.toml`의 `[whole]` 항목이 이를 지킵니다. `terms build`가 만든 `[whole]`
  항목이 필드 이름과 같은 낱말(`Type`)이면 필드까지 바꾸므로 지웁니다.

## 라이선스

원문은 공개 도메인에 헌정되었으며 그것이 불가능한 곳에서는 MIT-0으로
제공됩니다(`upstream/LICENSE.txt`). 이 번역도 같은 조건으로 제공하며 배포
사이트 루트에 원문 `LICENSE.txt`를 함께 싣습니다. 원문 페이지와 같이 Bikeshed가
만드는 CC0/OWFa 저작권 문단은 그대로 둡니다.

## 재현

```sh
cd projects/aqueous
../../target/release/yeokja translate upstream/spec        # 누락 세그먼트 번역 및 ko/ 재구성
python3 scripts/entities.py check                          # 원문이 &lt; &gt;를 쓴 세그먼트가 참조를 잃지 않았는지 확인
../../target/release/yeokja status --check upstream/spec
../../target/release/yeokja evaluate --mechanical-only upstream/spec
nix develop path:../../nix#aqueous -c ../../target/release/yeokja build html   # dist/site
```

번역하려면 인증된 Claude Code CLI가 필요합니다. `state/`는 번역의 진실의
원천이므로 커밋합니다. `ko/`, `build/`, `dist/`, `site/`는 `state/`와 원문에서
언제든 재생성되므로 커밋하지 않습니다. 번역문을 고칠 때는 `ko/`가 아니라
`state/`의 `translation` 필드를 고칩니다.

Nix devShell(`nix/projects/aqueous.nix`)은 python3와 uv를 제공하고, 셸에 들어갈 때
`requirements.txt`(`bikeshed==7.2.0`)를 `build/venv`에 설치합니다. nixpkgs의
bikeshed는 의존성이 깨져 있어 쓰지 않습니다. Bikeshed의 참조 데이터는 설치 뒤
`bikeshed update`로 한 번 받습니다(빌드는 `--no-update`로 돕니다).

빌드(`scripts/build.py`)는 다음 순서로 진행합니다.

1. 번역 파일 끝에 `<pre class=metadata>` 블록을 덧붙입니다: 한국어 제목(번역된
   `<h1>`과 같아야 합니다), `편집자 초안` 상태, 스냅샷 커밋 날짜, 비공식 번역
   경고 상자, 원문을 가리키는 `Translation` 항목. 원문 파일은 복사하거나 고치지
   않습니다.
2. 원문과 번역을 같은 Bikeshed로 `--die-on=warning`으로 빌드합니다. 번역이
   자동 링크나 정의를 깨뜨리면 여기서 멈춥니다.
3. `scripts/check.py`가 두 HTML의 요소 id 집합, 요소 다중집합, `<dfn>` 수, 수준별
   제목 수, 내부 링크 해소, 남은 `bikeshed:` 링크를 검사합니다.
4. Bikeshed가 영어로 만드는 보일러플레이트(요약, 목차, 색인, 참고 문헌, 편집자,
   사이드바 글자, 이슈·예시·그림·표 라벨, 머리말 날짜)만 한국어로 바꾸고
   `lang="ko"`로 고칩니다. 치환 횟수가 어긋나면 실패하므로 Bikeshed를 올릴 때
   드러납니다.
5. `site/index.html`과 `site/LICENSE.txt`를 만듭니다.

Deploy Pages 워크플로는 커밋된 번역 상태로 `ko/`를 재구성하고 `upstream/spec`의
미번역 여부를 검사한 뒤 빌드해, `dist-aqueous` 산출물을 Pages의 `aqueous/`에
배치합니다.
