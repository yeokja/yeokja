# 알고리즘카 (Алгоритмика 한국어 번역)

[Алгоритмика](https://ru.algorithmica.org)([algorithmica-org/algorithmica](https://github.com/algorithmica-org/algorithmica))의
러시아어판 — Sergey Slotin이 만들고 Tinkoff Generation의 CS 강의에서 쓰는 알고리즘·수학·머신러닝 자료 — 를
러시아어에서 한국어로 옮긴 비공식 번역입니다. [yeokja](https://github.com/yeokja/yeokja)와 함께 Anthropic 사의
`claude-opus-5-5` 모델을 활용하여 번역되었으며 학습을 모두 비허용한 상태로 작업하였습니다.

## 범위

- 원문: `upstream/` 서브모듈, 커밋 `ed1945c3d2de8548b1c744d3161eb668703db808`(2022-12-11, `master`)
- 번역 대상: `upstream/content/russian`의 공개 문서 272개(cs 152, math 53, ml 37, dl 28, 첫 화면과 `contributing.md`)
- 제외:
  - **영어판(`content/english`, *Algorithms for Modern Hardware*).** 저장소에 LICENSE가 없고, 러시아어판과 달리
    영어판에는 이용 허락 문구가 없습니다(`themes/algorithmica/i18n/en.toml`의 CC BY-NC 문구가 주석 처리됨).
    번역하지도, 빌드에 싣지도 않습니다.
  - **초안 111개(`draft: true`).** 원문 사이트에도 나오지 않습니다. 목록은 `scripts/scope.py`가 원문에서 계산해
    `yeokja.toml`의 `exclude`에 고정하며, `python3 scripts/scope.py check`가 원문과 어긋나면 실패합니다.
  - **`dl/aux/`.** reveal-hugo 슬라이드 묶음입니다. 번역 사이트는 슬라이드 테마를 쓰지 않습니다.
- 코드 블록(그 안의 러시아어 주석 포함), 인라인 코드, 수식(수식 속 `\text{…}` 포함), URL, 저자·편집자 이름,
  외부 문제 이름, 그림 속 글자는 원문 그대로입니다. 용어집은 `glossary.toml`(러시아어 격변화 형태를 함께 둠),
  프로젝트 용어 기준은 `terms.toml`에 있습니다.
- 원문 사이트처럼 math·ml·dl 분야의 섹션 머리말은 초안이라 첫 화면 목차에 없고, 그 글들은 검색과 링크로 닿습니다.

## 라이선스

러시아어판 자료는 [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/)으로 공개되어 있습니다
(사이트 하단 문구, `themes/algorithmica/i18n/ru.toml`의 `copyright2`). 이 번역은 원문(저작자 Sergey Slotin,
Tinkoff Generation, Algorithmica.org)을 수정한 개작물로서 같은 CC BY-SA 4.0으로 제공하며, 사이트 첫 화면과 하단에
원저작자·라이선스·번역 사실을 표시합니다. 원저작자의 공식 번역이 아닙니다.

- 사이트 테마 `themes/algorithmica`는 MIT 라이선스(`upstream/themes/algorithmica/LICENSE`)이며 그대로 싣습니다.
  번역 사이트에서 바꾼 레이아웃은 `overlay/layouts/`에 따로 둡니다.
- 원문 저자는 그림 일부의 출처를 알지 못한다고 밝혔습니다. 원문 사이트가 싣고 있는 그림을 수정 없이 함께
  싣습니다. 권리자의 요청이 있으면 해당 그림을 뺍니다.

## 재현

```sh
git submodule update --init projects/algorithmica/upstream
cd projects/algorithmica
python3 scripts/scope.py check                                       # exclude가 원문 초안 목록과 같은지
../../target/release/yeokja translate upstream/content/russian       # 누락 세그먼트 번역 및 ko/ 재구성
../../target/release/yeokja status --check upstream/content/russian
../../target/release/yeokja evaluate --mechanical-only upstream/content/russian
nix develop path:../../nix#algorithmica -c ../../target/release/yeokja build html   # dist/site
```

번역하려면 인증된 Claude Code CLI가 필요합니다. `state/`는 번역의 진실의 원천이므로 커밋합니다. `ko/`, `build/`,
`dist/`는 `state/`와 원문에서 언제든 재생성되므로 커밋하지 않습니다. CI(`.github/scripts/rebuild-translations.sh
algorithmica`)는 미번역 세그먼트를 먼저 검사한 뒤 state에서 `ko/`를 재구성하므로 번역 provider를 부르지 않습니다.

## 구성

- 파서: `mkdocs` — `$`/`$$` 수식을 원자로 가리고 front matter의 `title`을 번역합니다. 원문은 Hugo(Goldmark)
  사이트이지만 shortcode가 없어 이 파서로 충분합니다.
- 빌드 툴체인: `nix/projects/algorithmica.nix`(nixpkgs Hugo extended, python3). 원문 Netlify 빌드는 Hugo 0.96입니다.
- `overlay/config.yaml`: 한국어 한 언어 사이트(`content/korean`), `https://yeokja.moreal.dev/algorithmica/` 하위
  경로용 `canonifyURLs`, reveal-hugo 없이 `algorithmica` 테마만.
- `overlay/layouts/`, `overlay/i18n/ko.toml`: 방문 통계(Google Analytics, Yandex.Metrica) 제거, 한국어로 동작하지 않는
  lunr 검색을 부분 문자열 검색으로 교체, 쓰지 않는 TikZJax 제거, 편집 버튼 대신 원문(러시아어) 보기 버튼, 번역
  고지가 든 하단·첫 화면.
- `scripts/prepare_site.py`: 영어판·슬라이드 삭제, 그림과 초안 복사(초안 섹션이 원문처럼 숨도록), `*.ru.md` →
  `*.md`, Hugo 0.165가 받지 않는 `published:`·`date: {}` 줄 삭제, front matter `part:` 번역.
- `scripts/check_site.py`: 같은 설정으로 빌드한 러시아어 원문 사이트와 쪽 목록, 원문에 없던 깨진 로컬
  링크·앵커, 쪽별 제목·그림·코드 블록·`$` 수식 구분자 개수, 방문 통계 스크립트를 비교합니다. 원문에서도 깨진
  링크(옛 위키 문서의 `Категория:…` 링크 등) 25개는 원문 결함으로 보고만 합니다.
- 평가: `yeokja evaluate --mechanical-only`에 남는 10건은 확인된 오탐입니다. Alignment 4건은 숫자 0·1 같은 짧은
  토큰이 이웃 세그먼트와 겹친 것이고, Format 3세그먼트(각 2건)는 원문이 닫는 `$` 앞에 공백을 두어
  (`$f[l, r, k, p] = $ максимальная …`) 평가기가 사이의 러시아어를 수식으로 읽는 것입니다. KaTeX는 그 글을 본문으로
  그리므로 번역은 본문을 옮깁니다. 그중 모델이 러시아어를 남겼던 2세그먼트(`cs/games/nim.md`,
  `math/algebra/Операции_с_Матрицами.md`)는 state에서 직접 고쳤습니다.
