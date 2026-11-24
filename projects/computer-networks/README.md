# 컴퓨터 네트워크: 시스템 접근 (Computer Networks: A Systems Approach 한국어 번역)

Larry L. Peterson과 Bruce S. Davie의 [Computer Networks: A Systems Approach](https://book.systemsapproach.org/)
7판([SystemsApproach/7E](https://github.com/SystemsApproach/7E))을 한국어로 옮긴 비공식 번역입니다.
[yeokja](https://github.com/yeokja/yeokja)와 함께 Anthropic 사의 `claude-opus-5-5` 모델을 활용하여
번역되었으며 학습을 모두 비허용한 상태로 작업하였습니다.

## 범위

- 원문: `upstream/` 서브모듈, 커밋 `889b23414b71c4cda23a2d8a7aa2d294b419bccf`(2026-11-23, `main`.
  원문은 `main`을 늘 일관된 상태로 유지하고 웹판도 `main`에서 만듭니다)
- 번역 대상: `upstream/**/*.rst` 105개 파일. 최상위 장 문서와 그 장이 `.. include::`로 엮는 하위 디렉터리의
  절 파일, 목차의 "About This Book"인 `README.rst`
- 제외: `CONTRIBUTING.rst`와 `resources/README.rst`(원문 `conf.py`가 빌드에서 빼므로 사이트에 나오지 않습니다)
- 원문 그대로 두는 것: 그림(PNG·JPG 속 글자), 코드와 `literalinclude` 파일, 수식, `.. index::` 약어 항목
  (약어 목록 쪽), 더 읽을거리의 RFC·논문·책 제목, 사람 이름, URL
- 장 제목은 "12장: 신뢰성 있는 바이트 스트림"처럼 적고, 장 번호는 원문처럼 `chapters.rst`의 치환(`|TCP|` 등)으로
  들어갑니다. 그림·표 번호는 원문처럼 쪽마다 "그림 1.", "표 1." 꼴로 붙습니다. 제목을 옮겨도 쪽 안 앵커는 원문과
  같습니다(`scripts/en_anchors.py`).

## 라이선스

원문은 [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/)으로 공개되어 있습니다
(`upstream/LICENSE`, 각 파일의 SPDX 머리).

- *Computer Networks: A Systems Approach*, Larry Peterson and Bruce Davie, Copyright Larry L. Peterson and
  Bruce S. Davie 2026, <https://github.com/SystemsApproach/7E>, CC BY 4.0
- 이 책에 들어 있는 이전 판의 내용: *Computer Networks: A Systems Approach*, Larry Peterson and Bruce Davie,
  Copyright Elsevier 2012, <https://github.com/SystemsApproach/book>, CC BY 4.0

이 번역은 원문을 번역·수정한 개작물이며 같은 CC BY 4.0으로 제공합니다. 원문에서 바꾼 것은 본문 번역과
빌드 설정(`scripts/prepare.py`: 언어, 사이트 제목, 그림·표 번호 형식과 `:numref:` 레이블, 목차·약어 목록
레이블과 약어 목록 머리말, 맞춤법 검사·여러 버전 확장과 Google Analytics 스크립트 제거, 번역 고지)입니다. 사이트의 모든 쪽 위에 원저작자·원문 링크·라이선스·
번역 사실을 알리는 고지를 표시합니다. 이 번역은 원저작자가 검토하거나 보증하지 않았으며 Systems Approach의
공식 번역이 아닙니다. 확실하지 않을 때는 원문을 확인하십시오.

## 재현

```sh
cargo build --release -p yeokja-cli
git submodule update --init projects/computer-networks/upstream
cd projects/computer-networks
../../target/release/yeokja translate upstream
../../target/release/yeokja evaluate upstream --mechanical-only
../../target/release/yeokja status --check upstream
nix develop path:../../nix#computer-networks -c ../../target/release/yeokja build html   # dist/site
nix develop path:../../nix#computer-networks -c python3 scripts/check_site.py dist/site
nix develop path:../../nix#computer-networks -c python3 -m unittest discover -s scripts -p 'test_*.py'
```

`state/`는 번역의 진실의 원천이므로 커밋합니다. `ko/`, `build/`, `dist/`는 재생성되므로 커밋하지 않습니다
(`.gitignore`). CI(`.github/scripts/rebuild-translations.sh computer-networks`)는 state에서 `ko/`를
재구성만 하므로 번역 provider를 부르지 않습니다.

## 구성

- 파서: `rst`. 그림 캡션·범례와 `.. table::` 제목도 번역하고, 장 번호 치환(`|Intro|`) 옆에 한국어 조사가
  붙으면 RST가 읽을 수 있게 `\ `로 경계를 둡니다.
- 용어: `glossary.toml`(손으로 정한 용어)과 `terms.toml`(`yeokja terms build`가 고정한 반복 용어 420개와
  반복 세그먼트 2개). window 계열은 "윈도"로 적습니다. `glossary_plurals = true`라 영어 규칙 복수형은 단수형 항목이 함께 맞습니다.
- 빌드: `[derive]`가 `upstream/` 위에 `ko/`를 얹은 링크 트리를 만들고, `scripts/prepare.py`가 그 트리의
  `conf.py`에 한국어 설정 블록을 덧붙이고, `index.rst`·`_templates/layout.html`·`_templates/genindex.html`의
  목차·약어 목록 글자를 옮기고, `:numref:` 글자의 "Figure %s"·"Table %s"를 "그림 %s"·"표 %s"로 바꿉니다.
  원문 글자가 바뀌어 고정점을 찾지 못하면 실패합니다. 빌드는 `sphinx-build --fail-on-warning`이며
  툴체인은 `nix/projects/computer-networks.nix`(upstream `requirements.txt`를 `build/venv`에 설치)입니다.
- 검사: `scripts/check_site.py`가 같은 준비 단계를 거친 영어 원문 빌드(`build/reference`)와 한국어 사이트를
  쪽마다 비교합니다. 쪽 목록, 섹션 앵커와 순서, 제목·그림·표·이미지·번호 캡션·수식 블록 수, 깨진 로컬
  링크와 앵커, 분석 스크립트 잔재, 번역 고지 누락을 봅니다.

## upstream 갱신

자동으로 최신 커밋을 따라가지 않습니다. 새 커밋의 라이선스와 파일 범위를 검토한 뒤
`git -C projects/computer-networks/upstream checkout <커밋>`으로 gitlink를 바꾸고, `prepare.py`의
고정점이 그대로인지(`python3 -m unittest`), `status`, `translate`, `evaluate`, `build`, `check_site.py`를
다시 실행합니다. 원문을 올린 뒤에는 `yeokja reconcile`을 쓰지 않습니다.
